use crate::checkins::get_quantity;
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
    pub quantity: u32,
    pub done: bool,
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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WeekHabitWeekRow {
    pub id: String,
    pub name: String,
    pub period: String,
    pub target: u32,
    pub quantity: u32,
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
            let qty = get_quantity(db, &h.id, today);
            let done = qty >= h.target.quantity;
            today_rows.push(TodayHabitRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "day".to_string(),
                target: h.target.quantity,
                quantity: qty,
                done,
            });
        } else {
            let sum = week_sum_for_habit(db, h, &week_start)?;
            let done = sum >= h.target.quantity;
            today_rows.push(TodayHabitRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "week".to_string(),
                target: h.target.quantity,
                quantity: sum,
                done,
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
                let qty = get_quantity(db, &h.id, d);
                if qty >= h.target.quantity {
                    done_days += 1;
                }
            }
            week_rows.push(WeekHabitRow::Day(WeekHabitDayRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "day".to_string(),
                scheduled_days: scheduled,
                done_scheduled_days: done_days,
            }));
        } else {
            let sum = week_sum_for_habit(db, h, &week_start)?;
            week_rows.push(WeekHabitRow::Week(WeekHabitWeekRow {
                id: h.id.clone(),
                name: h.name.clone(),
                period: "week".to_string(),
                target: h.target.quantity,
                quantity: sum,
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
