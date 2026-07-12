use crate::date::parse_date_string;
use crate::error::CliError;
use crate::model::{
    Db, Routine, RoutineAction, RoutineActionKind, RoutineSession, RoutineSessionState,
    RoutineSessionStep, RoutineStep, RoutineStepStatus,
};
use crate::ts::validate_rfc3339;
use chrono::{DateTime, FixedOffset, NaiveTime};

fn normalize_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn validate_routine_name(name: &str) -> Result<String, CliError> {
    let n = normalize_name(name);
    if n.is_empty() {
        return Err(CliError::usage("Routine name is required"));
    }
    Ok(n)
}

pub fn stable_routine_sort(a: &Routine, b: &Routine) -> std::cmp::Ordering {
    let an = a.name.to_lowercase();
    let bn = b.name.to_lowercase();
    match an.cmp(&bn) {
        std::cmp::Ordering::Equal => a.id.cmp(&b.id),
        o => o,
    }
}

pub fn list_routines(db: &Db, include_archived: bool) -> Vec<Routine> {
    let mut out: Vec<Routine> = db
        .routines
        .iter()
        .filter(|r| include_archived || !r.archived)
        .cloned()
        .collect();
    out.sort_by(stable_routine_sort);
    out
}

pub fn select_routine_index(
    db: &Db,
    selector: &str,
    include_archived: bool,
) -> Result<usize, CliError> {
    let s = selector.trim();
    if s.is_empty() {
        return Err(CliError::usage("Routine selector is required"));
    }

    // Exact id: r0001
    if s.len() == 5 && s.starts_with('r') && s[1..].chars().all(|c| c.is_ascii_digit()) {
        let idx = db.routines.iter().position(|r| r.id == s);
        return match idx {
            Some(i) => {
                let r = &db.routines[i];
                if !include_archived && r.archived {
                    Err(CliError::not_found(format!(
                        "Routine not found: {}",
                        selector
                    )))
                } else {
                    Ok(i)
                }
            }
            None => Err(CliError::not_found(format!(
                "Routine not found: {}",
                selector
            ))),
        };
    }

    let prefix = s.to_lowercase();
    let mut matches: Vec<(usize, Routine)> = db
        .routines
        .iter()
        .enumerate()
        .filter(|(_, r)| include_archived || !r.archived)
        .filter(|(_, r)| r.name.to_lowercase().starts_with(&prefix))
        .map(|(i, r)| (i, r.clone()))
        .collect();

    matches.sort_by(|a, b| stable_routine_sort(&a.1, &b.1));

    if matches.is_empty() {
        return Err(CliError::not_found(format!(
            "Routine not found: {}",
            selector
        )));
    }

    if matches.len() > 1 {
        let candidates = matches
            .iter()
            .map(|(_, r)| format!("{} {}", r.id, r.name))
            .collect::<Vec<String>>()
            .join(", ");
        return Err(CliError::ambiguous(format!(
            "Ambiguous selector '{}'. Candidates: {}",
            selector, candidates
        )));
    }

    Ok(matches[0].0)
}

pub fn next_routine_id(db: &mut Db) -> String {
    let n = db.meta.next_routine_number;
    let id = format!("r{:04}", n);
    db.meta.next_routine_number = n + 1;
    id
}

fn validate_hhmm(value: &str, label: &str) -> Result<(), CliError> {
    let v = value.trim();
    if v.is_empty() {
        return Err(CliError::usage(format!("Invalid {}: (empty)", label)));
    }
    NaiveTime::parse_from_str(v, "%H:%M")
        .map(|_| ())
        .map_err(|_| CliError::usage(format!("Invalid {}: {}", label, value)))
}

pub fn make_routine(id: String, name: &str, at: Option<&str>, today: &str) -> Result<Routine, CliError> {
    let routine_name = validate_routine_name(name)?;
    if let Some(a) = at {
        validate_hhmm(a, "at")?;
    }

    Ok(Routine {
        id,
        name: routine_name,
        at: at.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        steps: Vec::new(),
        archived: false,
        created_date: today.to_string(),
        archived_date: None,
    })
}

pub fn add_step(
    routine: &mut Routine,
    name: &str,
    minutes: u32,
    quote: Option<&str>,
) -> Result<RoutineStep, CliError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(CliError::usage("Step name is required"));
    }
    if minutes < 1 {
        return Err(CliError::usage("Invalid minutes"));
    }

    let step = RoutineStep {
        index: routine.steps.len() as u32 + 1,
        name: n.to_string(),
        minutes,
        quote: quote.map(|q| q.trim().to_string()).filter(|q| !q.is_empty()),
    };
    routine.steps.push(step.clone());
    Ok(step)
}

fn local_date_from_ts(ts: &str) -> Result<String, CliError> {
    let dt: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(ts.trim())
        .map_err(|_| CliError::usage(format!("Invalid ts: {}", ts)))?;
    Ok(dt.date_naive().format("%Y-%m-%d").to_string())
}

fn session_number_for(db: &Db, routine_id: &str, date: &str) -> u32 {
    db.routine_sessions
        .iter()
        .filter(|s| s.routine_id == routine_id && s.date == date)
        .count() as u32
        + 1
}

pub fn start_session(
    db: &mut Db,
    routine: &Routine,
    date: &str,
    ts: &str,
) -> Result<RoutineSession, CliError> {
    parse_date_string(date, "date")?;
    validate_rfc3339(ts, "ts")?;

    let ts_date = local_date_from_ts(ts)?;
    if ts_date != date {
        return Err(CliError::usage(format!(
            "Invalid ts: date mismatch (date={}, ts_date={})",
            date, ts_date
        )));
    }

    if routine.steps.is_empty() {
        return Err(CliError::usage("Cannot start: routine has no steps"));
    }

    // idempotent start: same routine/date/started_ts => return existing
    if let Some(existing) = db
        .routine_sessions
        .iter()
        .find(|s| s.routine_id == routine.id && s.date == date && s.started_ts == ts.trim())
    {
        return Ok(existing.clone());
    }

    let n = session_number_for(db, &routine.id, date);
    let session_id = format!("rs:{}:{}:{}", routine.id, date, n);

    let steps: Vec<RoutineSessionStep> = routine
        .steps
        .iter()
        .map(|s| RoutineSessionStep {
            index: s.index,
            name: s.name.clone(),
            minutes: s.minutes,
            quote: s.quote.clone(),
            status: RoutineStepStatus::Pending,
            action_ts: None,
            skip_reason: None,
        })
        .collect();

    let session = RoutineSession {
        id: session_id,
        routine_id: routine.id.clone(),
        routine_name: routine.name.clone(),
        date: date.to_string(),
        started_ts: ts.trim().to_string(),
        state: RoutineSessionState::Active,
        steps,
        actions: Vec::new(),
    };

    db.routine_sessions.push(session.clone());
    Ok(session)
}

fn compact_ts(ts: &str) -> String {
    ts.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn action_id_for(session_id: &str, kind: RoutineActionKind, ts: &str) -> String {
    let k = match kind {
        RoutineActionKind::Next => "next",
        RoutineActionKind::Skip => "skip",
        RoutineActionKind::Done => "done",
    };
    format!("ra_{}_{}_{}", session_id.replace(':', "_"), k, compact_ts(ts))
}

pub fn select_session_index(db: &Db, selector: &str) -> Result<usize, CliError> {
    let s = selector.trim();
    if s.is_empty() {
        return Err(CliError::usage("Session selector is required"));
    }
    match db.routine_sessions.iter().position(|ss| ss.id == s) {
        Some(i) => Ok(i),
        None => Err(CliError::not_found(format!(
            "Routine session not found: {}",
            selector
        ))),
    }
}

fn first_pending_step_mut(session: &mut RoutineSession) -> Option<&mut RoutineSessionStep> {
    session.steps.iter_mut().find(|s| s.status == RoutineStepStatus::Pending)
}

pub fn apply_action(
    session: &mut RoutineSession,
    kind: RoutineActionKind,
    ts: &str,
    reason: Option<&str>,
) -> Result<Option<RoutineAction>, CliError> {
    validate_rfc3339(ts, "ts")?;

    let action_id = action_id_for(&session.id, kind, ts.trim());
    if session.actions.iter().any(|a| a.id == action_id) {
        return Ok(None); // idempotent no-op
    }

    match kind {
        RoutineActionKind::Next => {
            if session.state != RoutineSessionState::Active {
                return Err(CliError::usage("Session is not active"));
            }
            let step = first_pending_step_mut(session)
                .ok_or_else(|| CliError::usage("No pending steps (try `done`)"))?;
            step.status = RoutineStepStatus::Done;
            step.action_ts = Some(ts.trim().to_string());
            step.skip_reason = None;

            let action = RoutineAction {
                id: action_id,
                kind,
                ts: ts.trim().to_string(),
                step_index: Some(step.index),
                reason: None,
            };
            session.actions.push(action.clone());
            Ok(Some(action))
        }
        RoutineActionKind::Skip => {
            if session.state != RoutineSessionState::Active {
                return Err(CliError::usage("Session is not active"));
            }
            let step = first_pending_step_mut(session)
                .ok_or_else(|| CliError::usage("No pending steps (try `done`)"))?;
            step.status = RoutineStepStatus::Skipped;
            step.action_ts = Some(ts.trim().to_string());
            step.skip_reason = reason.map(|r| r.trim().to_string()).filter(|r| !r.is_empty());

            let action = RoutineAction {
                id: action_id,
                kind,
                ts: ts.trim().to_string(),
                step_index: Some(step.index),
                reason: reason.map(|r| r.trim().to_string()).filter(|r| !r.is_empty()),
            };
            session.actions.push(action.clone());
            Ok(Some(action))
        }
        RoutineActionKind::Done => {
            if session.state == RoutineSessionState::Done {
                return Ok(None);
            }

            if session
                .steps
                .iter()
                .any(|s| s.status == RoutineStepStatus::Pending)
            {
                return Err(CliError::usage("Cannot done: pending steps remain"));
            }

            session.state = RoutineSessionState::Done;
            let action = RoutineAction {
                id: action_id,
                kind,
                ts: ts.trim().to_string(),
                step_index: None,
                reason: None,
            };
            session.actions.push(action.clone());
            Ok(Some(action))
        }
    }
}

pub fn current_step(session: &RoutineSession) -> Option<&RoutineSessionStep> {
    session.steps.iter().find(|s| s.status == RoutineStepStatus::Pending)
}
