use crate::checkins::get_quantity;
use crate::completion::{counted_quantity, is_declared};
use crate::date::{date_range_inclusive, iso_week_end, iso_week_id, iso_week_start};
use crate::error::CliError;
use crate::habits::{is_scheduled_on, stable_habit_sort};
use crate::model::{Db, Habit};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Status {
    pub today: TodaySection,
    pub week: WeekSection,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TodaySection {
    pub date: String,
    pub habits: Vec<TodayHabitRow>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TodayHabitRow {
    pub id: String,
    pub name: String,
    pub period: String,
    pub target: u32,

    /// Quantity that counts toward completion semantics.
    pub quantity: u32,

    /// Raw recorded quantity (may differ if declaration-gated).
    pub raw_quantity: u32,

    pub done: bool,

    /// Whether this habit requires a declaration for completion.
    pub needs_declaration: bool,

    /// Whether a declaration exists for this date (if required).
    pub declared: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WeekSection {
    pub id: String,
    pub start_date: String,
    pub end_date: String,
    pub habits: Vec<WeekHabitRow>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum WeekHabitRow {
    Day(WeekHabitDayRow),
    Week(WeekHabitWeekRow),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WeekHabitDayRow {
    pub id: String,
    pub name: String,
    pub period: String,
    pub scheduled_days: u32,
    pub done_scheduled_days: u32,
    pub needs_declaration: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WeekHabitWeekRow {
    pub id: String,
    pub name: String,
    pub period: String,
    pub target: u32,

    /// Counted quantity for the ISO week.
    pub quantity: u32,

    /// Raw recorded quantity for the ISO week.
    pub raw_quantity: u32,

    pub needs_declaration: bool,
}

fn week_sum_for_habit(
    db: &Db,
    habit: &Habit,
    week_start_date: &str,
) -> Result<(u32, u32), CliError> {
    let end = iso_week_end(week_start_date)?;
    let days = date_range_inclusive(week_start_date, &end)?;
    let mut raw_sum = 0u32;
    let mut counted_sum = 0u32;
    for d in days {
        if d < habit.created_date {
            continue;
        }
        raw_sum = raw_sum.saturating_add(get_quantity(db, &habit.id, &d));
        counted_sum = counted_sum.saturating_add(counted_quantity(db, habit, &d));
    }
    Ok((raw_sum, counted_sum))
}

pub fn build_status(
    db: &Db,
    date: &str,
    week_of: Option<&str>,
    include_archived: bool,
) -> Result<Status, CliError> {
    let today = date;
    let week_start = iso_week_start(week_of.unwrap_or(today))?;
    let week_end = iso_week_end(&week_start)?;
    let week_id = iso_week_id(&week_start)?;

    let mut habits: Vec<Habit> = db
        .habits
        .iter()
        .filter(|h| include_archived || !h.archived)
        .cloned()
        .collect();
    habits.sort_by(stable_habit_sort);

    let mut today_rows: Vec<TodayHabitRow> = Vec::new();
    for h in habits.iter() {
        if !is_scheduled_on(h, today)? {
            continue;
        }
        if h.target.period == "day" {
            let raw = get_quantity(db, &h.id, today);
            let counted = counted_quantity(db, h, today);
            let declared = is_declared(db, h, today);
            let done = declared && counted >= h.target.quantity;
            today_rows.push(TodayHabitRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "day".to_string(),
                target: h.target.quantity,
                quantity: counted,
                raw_quantity: raw,
                done,
                needs_declaration: h.needs_declaration,
                declared,
            });
        } else {
            let (raw_sum, counted_sum) = week_sum_for_habit(db, h, &week_start)?;
            let done = counted_sum >= h.target.quantity;
            today_rows.push(TodayHabitRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "week".to_string(),
                target: h.target.quantity,
                quantity: counted_sum,
                raw_quantity: raw_sum,
                done,
                needs_declaration: h.needs_declaration,
                declared: is_declared(db, h, today),
            });
        }
    }

    let week_days = date_range_inclusive(&week_start, &week_end)?;

    let mut week_rows: Vec<WeekHabitRow> = Vec::new();
    for h in habits.iter() {
        if h.target.period == "day" {
            let mut scheduled = 0u32;
            let mut done_days = 0u32;
            for d in week_days.iter() {
                if !is_scheduled_on(h, d)? {
                    continue;
                }
                scheduled += 1;
                let counted = counted_quantity(db, h, d);
                let declared = is_declared(db, h, d);
                if declared && counted >= h.target.quantity {
                    done_days += 1;
                }
            }
            week_rows.push(WeekHabitRow::Day(WeekHabitDayRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "day".to_string(),
                scheduled_days: scheduled,
                done_scheduled_days: done_days,
                needs_declaration: h.needs_declaration,
            }));
        } else {
            let (raw_sum, counted_sum) = week_sum_for_habit(db, h, &week_start)?;
            week_rows.push(WeekHabitRow::Week(WeekHabitWeekRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "week".to_string(),
                target: h.target.quantity,
                quantity: counted_sum,
                raw_quantity: raw_sum,
                needs_declaration: h.needs_declaration,
            }));
        }
    }

    Ok(Status {
        today: TodaySection {
            date: today.to_string(),
            habits: today_rows,
        },
        week: WeekSection {
            id: week_id,
            start_date: week_start,
            end_date: week_end,
            habits: week_rows,
        },
    })
}
