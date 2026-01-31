//! HelloHabit-style recap: per-habit completion percentage over a time range.
//!
//! Completion is calculated as:
//! - **Daily habits**: successes / eligible_days where success = counted_quantity >= target
//! - **Weekly habits**: successful_weeks / eligible_weeks where success = week_sum >= target
//!
//! This matches the semantics used in `stats.rs`.

use crate::completion::counted_quantity;
use crate::date::{add_days, date_range_inclusive, iso_week_end, iso_week_start};
use crate::error::CliError;
use crate::habits::{is_scheduled_on, stable_habit_sort};
use crate::model::{Db, Habit};

/// Supported recap time ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecapRange {
    /// Year-to-date: Jan 1 of current year through today
    Ytd,
    /// Past 30 days (including today)
    Month,
    /// Past 7 days (including today)
    Week,
}

impl RecapRange {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecapRange::Ytd => "ytd",
            RecapRange::Month => "month",
            RecapRange::Week => "week",
        }
    }
}

/// A single recap row for one habit.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecapRow {
    pub habit_id: String,
    pub name: String,
    /// "day" or "week"
    pub period: String,
    /// Human-readable target label, e.g. "8 cups/day" or "3/week"
    pub target_label: String,
    /// Raw target quantity
    pub target: u32,
    /// Number of successes (days or weeks where target was met)
    pub successes: u32,
    /// Number of eligible periods (scheduled days or eligible weeks)
    pub eligible: u32,
    /// Completion percentage as 0.0-1.0 (None if eligible=0)
    pub rate: Option<f64>,
    /// Completion percentage as 0-100 integer (None if eligible=0)
    pub percent: Option<u32>,
    /// The time range used
    pub range: RecapRangeInfo,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecapRangeInfo {
    pub kind: String,
    pub from: String,
    pub to: String,
}

/// Compute the from/to dates for a given range relative to `today`.
pub fn compute_range_dates(range: RecapRange, today: &str) -> Result<(String, String), CliError> {
    match range {
        RecapRange::Ytd => {
            // Jan 1 of the year in `today`
            let year = &today[0..4];
            let from = format!("{}-01-01", year);
            Ok((from, today.to_string()))
        }
        RecapRange::Month => {
            // Past 30 days including today (so today - 29 days)
            let from = add_days(today, -29)?;
            Ok((from, today.to_string()))
        }
        RecapRange::Week => {
            // Past 7 days including today (so today - 6 days)
            let from = add_days(today, -6)?;
            Ok((from, today.to_string()))
        }
    }
}

/// Compute daily habit recap stats over a date range.
fn compute_daily_recap(
    db: &Db,
    habit: &Habit,
    from: &str,
    to: &str,
    range: RecapRange,
) -> Result<RecapRow, CliError> {
    let days = date_range_inclusive(from, to)?;

    // Filter to scheduled days within the habit's existence
    let scheduled_days: Vec<String> = days
        .into_iter()
        .filter(|d| {
            d.as_str() >= habit.created_date.as_str()
                && is_scheduled_on(habit, d).unwrap_or(false)
        })
        .collect();

    let eligible = scheduled_days.len() as u32;
    let successes = scheduled_days
        .iter()
        .filter(|d| counted_quantity(db, habit, d) >= habit.target.quantity)
        .count() as u32;

    let rate = if eligible == 0 {
        None
    } else {
        Some(successes as f64 / eligible as f64)
    };

    let percent = rate.map(|r| (r * 100.0).round() as u32);

    let target_label = if habit.target.quantity == 1 {
        "1/day".to_string()
    } else {
        format!("{}/day", habit.target.quantity)
    };

    Ok(RecapRow {
        habit_id: habit.id.clone(),
        name: habit.name.clone(),
        period: "day".to_string(),
        target_label,
        target: habit.target.quantity,
        successes,
        eligible,
        rate,
        percent,
        range: RecapRangeInfo {
            kind: range.as_str().to_string(),
            from: from.to_string(),
            to: to.to_string(),
        },
    })
}

/// Sum of counted quantities for a habit over a week.
fn week_sum_for_habit(db: &Db, habit: &Habit, week_start_date: &str) -> Result<u32, CliError> {
    let end = iso_week_end(week_start_date)?;
    let days = date_range_inclusive(week_start_date, &end)?;

    let mut sum = 0u32;
    for d in days {
        if d < habit.created_date {
            continue;
        }
        sum = sum.saturating_add(counted_quantity(db, habit, &d));
    }
    Ok(sum)
}

/// Generate all ISO week start dates from `from_week_start` to `to_week_start`.
fn week_range_inclusive(from_week_start: &str, to_week_start: &str) -> Result<Vec<String>, CliError> {
    let mut weeks = Vec::new();
    let mut cur = from_week_start.to_string();
    while cur.as_str() <= to_week_start {
        weeks.push(cur.clone());
        cur = add_days(&cur, 7)?;
    }
    Ok(weeks)
}

/// Compute weekly habit recap stats over a date range.
fn compute_weekly_recap(
    db: &Db,
    habit: &Habit,
    from: &str,
    to: &str,
    range: RecapRange,
) -> Result<RecapRow, CliError> {
    let start_week = iso_week_start(from)?;
    let end_week = iso_week_start(to)?;
    let all_week_starts = week_range_inclusive(&start_week, &end_week)?;

    // Filter to weeks where the habit existed by end of week
    let eligible_week_starts: Vec<String> = all_week_starts
        .into_iter()
        .filter(|ws| iso_week_end(ws).map(|e| e >= habit.created_date).unwrap_or(false))
        .collect();

    let eligible = eligible_week_starts.len() as u32;
    let successes = eligible_week_starts
        .iter()
        .filter(|ws| {
            week_sum_for_habit(db, habit, ws)
                .map(|sum| sum >= habit.target.quantity)
                .unwrap_or(false)
        })
        .count() as u32;

    let rate = if eligible == 0 {
        None
    } else {
        Some(successes as f64 / eligible as f64)
    };

    let percent = rate.map(|r| (r * 100.0).round() as u32);

    let target_label = if habit.target.quantity == 1 {
        "1/week".to_string()
    } else {
        format!("{}/week", habit.target.quantity)
    };

    Ok(RecapRow {
        habit_id: habit.id.clone(),
        name: habit.name.clone(),
        period: "week".to_string(),
        target_label,
        target: habit.target.quantity,
        successes,
        eligible,
        rate,
        percent,
        range: RecapRangeInfo {
            kind: range.as_str().to_string(),
            from: from.to_string(),
            to: to.to_string(),
        },
    })
}

/// Build recap rows for given habits over the specified range.
pub fn build_recap(
    db: &Db,
    habits: &[Habit],
    range: RecapRange,
    today: &str,
) -> Result<Vec<RecapRow>, CliError> {
    let (from, to) = compute_range_dates(range, today)?;

    let mut sorted: Vec<Habit> = habits.to_vec();
    sorted.sort_by(stable_habit_sort);

    let mut rows = Vec::new();
    for h in sorted.iter() {
        let row = if h.target.period == "day" {
            compute_daily_recap(db, h, &from, &to, range)?
        } else {
            compute_weekly_recap(db, h, &from, &to, range)?
        };
        rows.push(row);
    }

    // Sort by percentage descending (None at bottom), then by name
    rows.sort_by(|a, b| {
        match (a.percent, b.percent) {
            (Some(ap), Some(bp)) => bp.cmp(&ap), // Descending
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(rows)
}

/// Render a progress bar for table output.
pub fn render_progress_bar(percent: Option<u32>, width: usize) -> String {
    match percent {
        None => "-".repeat(width),
        Some(p) => {
            let filled = ((p as f64 / 100.0) * width as f64).round() as usize;
            let filled = filled.min(width);
            let empty = width - filled;
            format!("{}{}", "█".repeat(filled), "░".repeat(empty))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_range_dates_ytd() {
        let (from, to) = compute_range_dates(RecapRange::Ytd, "2026-03-15").unwrap();
        assert_eq!(from, "2026-01-01");
        assert_eq!(to, "2026-03-15");
    }

    #[test]
    fn test_compute_range_dates_month() {
        let (from, to) = compute_range_dates(RecapRange::Month, "2026-01-31").unwrap();
        assert_eq!(from, "2026-01-02");
        assert_eq!(to, "2026-01-31");
    }

    #[test]
    fn test_compute_range_dates_week() {
        let (from, to) = compute_range_dates(RecapRange::Week, "2026-01-31").unwrap();
        assert_eq!(from, "2026-01-25");
        assert_eq!(to, "2026-01-31");
    }

    #[test]
    fn test_render_progress_bar() {
        assert_eq!(render_progress_bar(Some(100), 10), "██████████");
        assert_eq!(render_progress_bar(Some(50), 10), "█████░░░░░");
        assert_eq!(render_progress_bar(Some(0), 10), "░░░░░░░░░░");
        assert_eq!(render_progress_bar(None, 10), "----------");
    }
}
