use crate::date::{iso_week_start, parse_date_string};
use crate::error::CliError;
use crate::model::{Db, Excuse, ExcuseKind};
use crate::ts::validate_rfc3339;

pub fn next_excuse_id(db: &mut Db) -> String {
    let n = db.meta.next_excuse_number;
    let id = format!("e{:06}", n);
    db.meta.next_excuse_number = n + 1;
    id
}

pub fn allowed_excuses_used_in_week(
    db: &Db,
    habit_id: &str,
    week_start: &str,
) -> Result<u32, CliError> {
    let week_end = crate::date::iso_week_end(week_start)?;
    Ok(db
        .excuses
        .iter()
        .filter(|e| e.habit_id == habit_id)
        .filter(|e| e.kind == ExcuseKind::Allowed)
        .filter(|e| e.date.as_str() >= week_start && e.date.as_str() <= week_end.as_str())
        .count() as u32)
}

pub fn excuse(
    db: &mut Db,
    habit_id: &str,
    date: &str,
    ts: &str,
    kind_requested: ExcuseKind,
    reason: &str,
    quota_per_week: u32,
) -> Result<(Excuse, u32, u32), CliError> {
    parse_date_string(date, "date")?;
    validate_rfc3339(ts, "ts")?;

    let r = reason.trim();
    if r.is_empty() {
        return Err(CliError::usage("Excuse reason is required"));
    }

    let week_start = iso_week_start(date)?;
    let used = allowed_excuses_used_in_week(db, habit_id, &week_start)?;
    let remaining = quota_per_week.saturating_sub(used);

    // Deterministic policy: if quota exceeded, record as denied regardless of request.
    let kind = if kind_requested == ExcuseKind::Allowed && remaining == 0 {
        ExcuseKind::Denied
    } else {
        kind_requested
    };

    let id = next_excuse_id(db);
    let ex = Excuse {
        id,
        habit_id: habit_id.to_string(),
        date: date.to_string(),
        ts: ts.trim().to_string(),
        kind,
        reason: r.to_string(),
    };
    db.excuses.push(ex.clone());

    // Recompute used/remaining if we recorded allowed.
    let used2 = if kind == ExcuseKind::Allowed {
        used + 1
    } else {
        used
    };
    let remaining2 = quota_per_week.saturating_sub(used2);

    Ok((ex, used2, remaining2))
}

pub fn has_allowed_excuse(db: &Db, habit_id: &str, date: &str) -> bool {
    db.excuses
        .iter()
        .any(|e| e.habit_id == habit_id && e.date == date && e.kind == ExcuseKind::Allowed)
}
