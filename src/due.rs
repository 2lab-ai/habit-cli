use crate::completion::{counted_quantity, is_declared};
use crate::date::{iso_week_end, iso_week_start, date_range_inclusive};
use crate::error::CliError;
use crate::habits::{is_scheduled_on, stable_habit_sort};
use crate::model::{Db, Habit};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DueOutput {
    pub date: String,
    pub due: Vec<DueHabitRow>,
    pub counts: DueCounts,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DueHabitRow {
    pub id: String,
    pub name: String,
    pub period: String,
    pub target: u32,
    pub quantity: u32,
    pub remaining: u32,
    pub scheduled: bool,
    pub done: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DueCounts {
    pub due: u32,
}

fn week_sum_for_habit(
    db: &Db,
    habit: &Habit,
    week_start_date: &str,
) -> Result<u32, CliError> {
    let end = iso_week_end(week_start_date)?;
    let days = date_range_inclusive(week_start_date, &end)?;
    let mut counted_sum = 0u32;
    for d in days {
        if d < habit.created_date {
            continue;
        }
        counted_sum = counted_sum.saturating_add(counted_quantity(db, habit, &d));
    }
    Ok(counted_sum)
}

pub fn build_due(
    db: &Db,
    date: &str,
    include_archived: bool,
) -> Result<DueOutput, CliError> {
    let week_start = iso_week_start(date)?;

    // Collect and sort habits
    let mut habits: Vec<Habit> = db
        .habits
        .iter()
        .filter(|h| include_archived || !h.archived)
        .cloned()
        .collect();
    habits.sort_by(stable_habit_sort);

    let mut due_rows: Vec<DueHabitRow> = Vec::new();

    for h in habits.iter() {
        let scheduled = is_scheduled_on(h, date)?;
        if !scheduled {
            continue;
        }

        if h.target.period == "day" {
            let counted = counted_quantity(db, h, date);
            let declared = is_declared(db, h, date);
            let done = declared && counted >= h.target.quantity;

            // Only include if not done
            if !done {
                let remaining = h.target.quantity.saturating_sub(counted);
                due_rows.push(DueHabitRow {
                    id: h.id.clone(),
                    name: h.name.clone(),
                    period: "day".to_string(),
                    target: h.target.quantity,
                    quantity: counted,
                    remaining,
                    scheduled: true,
                    done: false,
                });
            }
        } else {
            // Weekly habit
            let counted_sum = week_sum_for_habit(db, h, &week_start)?;
            let done = counted_sum >= h.target.quantity;

            // Only include if not done
            if !done {
                let remaining = h.target.quantity.saturating_sub(counted_sum);
                due_rows.push(DueHabitRow {
                    id: h.id.clone(),
                    name: h.name.clone(),
                    period: "week".to_string(),
                    target: h.target.quantity,
                    quantity: counted_sum,
                    remaining,
                    scheduled: true,
                    done: false,
                });
            }
        }
    }

    let due_count = due_rows.len() as u32;

    Ok(DueOutput {
        date: date.to_string(),
        due: due_rows,
        counts: DueCounts { due: due_count },
    })
}
