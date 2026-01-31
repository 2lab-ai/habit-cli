use crate::date::iso_weekday;
use crate::error::CliError;
use crate::model::{Db, Habit, Target};
use crate::schedule::{parse_schedule_pattern, validate_schedule, Schedule};

fn normalize_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn validate_habit_name(name: &str) -> Result<String, CliError> {
    let n = normalize_name(name);
    if n.is_empty() {
        return Err(CliError::usage("Habit name is required"));
    }
    Ok(n)
}

pub fn next_habit_id(db: &mut Db) -> String {
    let n = db.meta.next_habit_number;
    let id = format!("h{:04}", n);
    db.meta.next_habit_number = n + 1;
    id
}

pub fn stable_habit_sort(a: &Habit, b: &Habit) -> std::cmp::Ordering {
    let an = a.name.to_lowercase();
    let bn = b.name.to_lowercase();
    match an.cmp(&bn) {
        std::cmp::Ordering::Equal => a.id.cmp(&b.id),
        o => o,
    }
}

pub fn list_habits(db: &Db, include_archived: bool) -> Vec<Habit> {
    let mut out: Vec<Habit> = db
        .habits
        .iter()
        .filter(|h| include_archived || !h.archived)
        .cloned()
        .collect();
    out.sort_by(stable_habit_sort);
    out
}

pub fn select_habit_index(
    db: &Db,
    selector: &str,
    include_archived: bool,
) -> Result<usize, CliError> {
    let s = selector.trim();
    if s.is_empty() {
        return Err(CliError::usage("Habit selector is required"));
    }

    if s.len() == 5 && s.starts_with('h') && s[1..].chars().all(|c| c.is_ascii_digit()) {
        let idx = db.habits.iter().position(|h| h.id == s);
        return match idx {
            Some(i) => {
                let h = &db.habits[i];
                if !include_archived && h.archived {
                    Err(CliError::not_found(format!(
                        "Habit not found: {}",
                        selector
                    )))
                } else {
                    Ok(i)
                }
            }
            None => Err(CliError::not_found(format!(
                "Habit not found: {}",
                selector
            ))),
        };
    }

    let prefix = s.to_lowercase();
    let mut matches: Vec<(usize, Habit)> = db
        .habits
        .iter()
        .enumerate()
        .filter(|(_, h)| include_archived || !h.archived)
        .filter(|(_, h)| h.name.to_lowercase().starts_with(&prefix))
        .map(|(i, h)| (i, h.clone()))
        .collect();

    matches.sort_by(|a, b| stable_habit_sort(&a.1, &b.1));

    if matches.is_empty() {
        return Err(CliError::not_found(format!(
            "Habit not found: {}",
            selector
        )));
    }

    if matches.len() > 1 {
        let candidates = matches
            .iter()
            .map(|(_, h)| format!("{} {}", h.id, h.name))
            .collect::<Vec<String>>()
            .join(", ");
        return Err(CliError::ambiguous(format!(
            "Ambiguous selector '{}'. Candidates: {}",
            selector, candidates
        )));
    }

    Ok(matches[0].0)
}

pub fn make_habit(
    id: String,
    name: &str,
    schedule_pattern: &str,
    period: &str,
    target: u32,
    notes: Option<&str>,
    today: &str,
) -> Result<Habit, CliError> {
    let habit_name = validate_habit_name(name)?;
    let schedule: Schedule = parse_schedule_pattern(schedule_pattern)?;
    validate_schedule(&schedule)?;

    if period != "day" && period != "week" {
        return Err(CliError::usage(format!("Invalid period: {}", period)));
    }

    if target < 1 {
        return Err(CliError::usage("Invalid target"));
    }

    let notes = notes.map(|s| s.to_string());

    Ok(Habit {
        id,
        name: habit_name,
        schedule,
        target: Target {
            period: period.to_string(),
            quantity: target,
        },
        notes,
        archived: false,
        created_date: today.to_string(),
        archived_date: None,
    })
}

pub fn is_scheduled_on(habit: &Habit, date: &str) -> Result<bool, CliError> {
    if date < habit.created_date.as_str() {
        return Ok(false);
    }
    let wd = iso_weekday(date)?;
    Ok(habit.schedule.days.contains(&wd))
}
