use crate::date::parse_date_string;
use crate::error::CliError;
use crate::model::{Checkin, Db};
use std::collections::BTreeSet;

fn find_checkin_index(db: &Db, habit_id: &str, date: &str) -> Option<usize> {
    db.checkins
        .iter()
        .position(|c| c.habit_id == habit_id && c.date == date)
}

pub fn get_quantity(db: &Db, habit_id: &str, date: &str) -> u32 {
    match find_checkin_index(db, habit_id, date) {
        Some(i) => db.checkins[i].quantity,
        None => 0,
    }
}

pub fn set_quantity(db: &mut Db, habit_id: &str, date: &str, quantity: u32) -> Result<(), CliError> {
    parse_date_string(date, "date")?;

    let idx = find_checkin_index(db, habit_id, date);

    if quantity == 0 {
        if let Some(i) = idx {
            db.checkins.remove(i);
        }
        return Ok(());
    }

    match idx {
        None => db.checkins.push(Checkin {
            habit_id: habit_id.to_string(),
            date: date.to_string(),
            quantity,
        }),
        Some(i) => db.checkins[i].quantity = quantity,
    }

    Ok(())
}

pub fn add_quantity(db: &mut Db, habit_id: &str, date: &str, delta: u32) -> Result<u32, CliError> {
    parse_date_string(date, "date")?;
    if delta < 1 {
        return Err(CliError::usage("Invalid quantity"));
    }

    let cur = get_quantity(db, habit_id, date);
    let total = cur.saturating_add(delta);
    set_quantity(db, habit_id, date, total)?;
    Ok(total)
}

pub fn list_checkins_for_habit(db: &Db, habit_id: &str) -> Vec<Checkin> {
    let mut out: Vec<Checkin> = db
        .checkins
        .iter()
        .filter(|c| c.habit_id == habit_id)
        .cloned()
        .collect();

    out.sort_by(|a, b| {
        if a.date != b.date {
            a.date.cmp(&b.date)
        } else {
            a.habit_id.cmp(&b.habit_id)
        }
    });

    out
}

pub fn list_checkins_in_range(
    db: &Db,
    from: Option<&str>,
    to: Option<&str>,
    habit_ids: Option<&BTreeSet<String>>,
) -> Vec<Checkin> {
    let mut out: Vec<Checkin> = db
        .checkins
        .iter()
        .filter(|c| {
            if let Some(ids) = habit_ids {
                if !ids.contains(&c.habit_id) {
                    return false;
                }
            }
            if let Some(f) = from {
                if c.date < f {
                    return false;
                }
            }
            if let Some(t) = to {
                if c.date > t {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    out.sort_by(|a, b| {
        if a.date != b.date {
            a.date.cmp(&b.date)
        } else {
            a.habit_id.cmp(&b.habit_id)
        }
    });

    out
}
