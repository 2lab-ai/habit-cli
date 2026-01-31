use crate::checkins::get_quantity;
use crate::date::{add_days, date_range_inclusive, iso_week_end, iso_week_start};
use crate::error::CliError;
use crate::habits::{is_scheduled_on, stable_habit_sort};
use crate::model::{Db, Habit};

#[derive(Debug, Clone, serde::Serialize)]
pub struct StatsRow {
    pub habit_id: String,
    pub name: String,
    pub period: String,
    pub target: u32,
    pub window: Window,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub success_rate: SuccessRate,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Window {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SuccessRate {
    pub successes: u32,
    pub eligible: u32,
    pub rate: Option<f64>,
}

fn compute_daily_stats(db: &Db, habit: &Habit, from: &str, to: &str) -> Result<StatsRow, CliError> {
    let days = date_range_inclusive(from, to)?;

    let mut scheduled_days: Vec<String> = Vec::new();
    for d in days.iter() {
        if is_scheduled_on(habit, d)? {
            scheduled_days.push(d.clone());
        }
    }

    let mut successes = 0u32;
    for d in scheduled_days.iter() {
        if get_quantity(db, &habit.id, d) >= habit.target.quantity {
            successes += 1;
        }
    }

    let eligible = scheduled_days.len() as u32;
    let rate = if eligible == 0 {
        None
    } else {
        Some(successes as f64 / eligible as f64)
    };

    let mut current = 0u32;
    for d in scheduled_days.iter().rev() {
        let ok = get_quantity(db, &habit.id, d) >= habit.target.quantity;
        if !ok {
            break;
        }
        current += 1;
    }

    let mut longest = 0u32;
    let mut run = 0u32;
    for d in scheduled_days.iter() {
        let ok = get_quantity(db, &habit.id, d) >= habit.target.quantity;
        if ok {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }

    Ok(StatsRow {
        habit_id: habit.id.clone(),
        name: habit.name.clone(),
        period: "day".to_string(),
        target: habit.target.quantity,
        window: Window {
            from: from.to_string(),
            to: to.to_string(),
        },
        current_streak: current,
        longest_streak: longest,
        success_rate: SuccessRate {
            successes,
            eligible,
            rate,
        },
    })
}

fn week_sum_for_habit(db: &Db, habit: &Habit, week_start_date: &str) -> Result<u32, CliError> {
    let end = iso_week_end(week_start_date)?;
    let days = date_range_inclusive(week_start_date, &end)?;

    let mut sum = 0u32;
    for d in days {
        if d < habit.created_date {
            continue;
        }
        sum = sum.saturating_add(get_quantity(db, &habit.id, &d));
    }
    Ok(sum)
}

fn week_range_inclusive(
    from_week_start: &str,
    to_week_start: &str,
) -> Result<Vec<String>, CliError> {
    let mut weeks: Vec<String> = Vec::new();
    let mut cur = from_week_start.to_string();
    while cur <= to_week_start {
        weeks.push(cur.clone());
        cur = add_days(&cur, 7)?;
    }
    Ok(weeks)
}

fn compute_weekly_stats(
    db: &Db,
    habit: &Habit,
    from: &str,
    to: &str,
) -> Result<StatsRow, CliError> {
    let start_week = iso_week_start(from)?;
    let end_week = iso_week_start(to)?;
    let all_week_starts = week_range_inclusive(&start_week, &end_week)?;

    let mut eligible_week_starts: Vec<String> = Vec::new();
    for ws in all_week_starts.iter() {
        if iso_week_end(ws)? >= habit.created_date {
            eligible_week_starts.push(ws.clone());
        }
    }

    let mut successes = 0u32;
    for ws in eligible_week_starts.iter() {
        if week_sum_for_habit(db, habit, ws)? >= habit.target.quantity {
            successes += 1;
        }
    }

    let eligible = eligible_week_starts.len() as u32;
    let rate = if eligible == 0 {
        None
    } else {
        Some(successes as f64 / eligible as f64)
    };

    let mut current = 0u32;
    for ws in eligible_week_starts.iter().rev() {
        let ok = week_sum_for_habit(db, habit, ws)? >= habit.target.quantity;
        if !ok {
            break;
        }
        current += 1;
    }

    let mut longest = 0u32;
    let mut run = 0u32;
    for ws in eligible_week_starts.iter() {
        let ok = week_sum_for_habit(db, habit, ws)? >= habit.target.quantity;
        if ok {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }

    Ok(StatsRow {
        habit_id: habit.id.clone(),
        name: habit.name.clone(),
        period: "week".to_string(),
        target: habit.target.quantity,
        window: Window {
            from: from.to_string(),
            to: to.to_string(),
        },
        current_streak: current,
        longest_streak: longest,
        success_rate: SuccessRate {
            successes,
            eligible,
            rate,
        },
    })
}

pub fn build_stats(
    db: &Db,
    habits: &[Habit],
    from: &str,
    to: &str,
) -> Result<Vec<StatsRow>, CliError> {
    let mut sorted: Vec<Habit> = habits.to_vec();
    sorted.sort_by(stable_habit_sort);

    let mut rows = Vec::new();
    for h in sorted.iter() {
        if h.target.period == "day" {
            rows.push(compute_daily_stats(db, h, from, to)?);
        } else {
            rows.push(compute_weekly_stats(db, h, from, to)?);
        }
    }

    Ok(rows)
}
