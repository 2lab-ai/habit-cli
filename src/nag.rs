use crate::date::parse_date_string;
use crate::due::build_due;
use crate::error::CliError;
use crate::model::{Db, NagConfig, NagState};
use crate::penalty::outstanding_debts_as_of;
use crate::ts::validate_rfc3339;
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveTime, SecondsFormat, TimeZone};

fn parse_hhmm(value: &str, label: &str) -> Result<NaiveTime, CliError> {
    let v = value.trim();
    if v.is_empty() {
        return Err(CliError::usage(format!("Invalid {}: (empty)", label)));
    }
    NaiveTime::parse_from_str(v, "%H:%M")
        .map_err(|_| CliError::usage(format!("Invalid {}: {}", label, value)))
}

fn parse_now(ts: &str) -> Result<DateTime<FixedOffset>, CliError> {
    validate_rfc3339(ts, "now_ts")?;
    DateTime::parse_from_rfc3339(ts.trim())
        .map_err(|_| CliError::usage(format!("Invalid now_ts: {}", ts)))
}

fn is_within_quiet_hours(now_t: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start == end {
        return false;
    }
    if start < end {
        now_t >= start && now_t < end
    } else {
        // wraps midnight (e.g., 23:00–08:00)
        now_t >= start || now_t < end
    }
}

fn quiet_end_at(now: DateTime<FixedOffset>, start: NaiveTime, end: NaiveTime) -> DateTime<FixedOffset> {
    let now_t = now.time();
    let wraps = start > end;

    let base_date: NaiveDate = now.date_naive();
    let end_date = if !wraps {
        base_date
    } else if now_t < end {
        base_date
    } else {
        base_date + Duration::days(1)
    };

    let naive = end_date.and_time(end);
    now.offset()
        .from_local_datetime(&naive)
        .single()
        .unwrap_or(now)
}

fn fmt_rfc3339(ts: DateTime<FixedOffset>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NagPlan {
    pub date: String,
    pub now_ts: String,
    pub quiet_start: String,
    pub quiet_end: String,
    pub cadence_minutes: u32,
    pub snoozed_until: Option<String>,
    pub last_sent_ts: Option<String>,
    pub due_count: u32,
    pub debts_due_count: u32,
    pub severity: u32,
    pub should_send: bool,
    pub next_check_at: String,
}

pub fn set_config(
    db: &mut Db,
    quiet_start: &str,
    quiet_end: &str,
    cadence_minutes: Option<u32>,
) -> Result<NagConfig, CliError> {
    let qs = quiet_start.trim();
    let qe = quiet_end.trim();
    let _ = parse_hhmm(qs, "quiet_start")?;
    let _ = parse_hhmm(qe, "quiet_end")?;

    if let Some(c) = cadence_minutes {
        if c < 1 {
            return Err(CliError::usage("Invalid cadence_minutes"));
        }
        db.nag.config.cadence_minutes = c;
    }

    db.nag.config.quiet_start = qs.to_string();
    db.nag.config.quiet_end = qe.to_string();

    Ok(db.nag.config.clone())
}

pub fn snooze(db: &mut Db, until_ts: &str, reason: Option<&str>) -> Result<NagState, CliError> {
    validate_rfc3339(until_ts, "until")?;
    db.nag.state.snoozed_until = Some(until_ts.trim().to_string());
    db.nag.state.snooze_reason = reason
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty());
    Ok(db.nag.state.clone())
}

pub fn unsnooze(db: &mut Db) -> NagState {
    db.nag.state.snoozed_until = None;
    db.nag.state.snooze_reason = None;
    db.nag.state.clone()
}

pub fn record_sent(db: &mut Db, ts: &str) -> Result<NagState, CliError> {
    validate_rfc3339(ts, "ts")?;
    db.nag.state.last_sent_ts = Some(ts.trim().to_string());
    Ok(db.nag.state.clone())
}

pub fn plan(db: &Db, date: &str, now_ts: &str, include_archived: bool) -> Result<NagPlan, CliError> {
    parse_date_string(date, "date")?;
    let now = parse_now(now_ts)?;

    let qs_t = parse_hhmm(&db.nag.config.quiet_start, "quiet_start")?;
    let qe_t = parse_hhmm(&db.nag.config.quiet_end, "quiet_end")?;

    let due = build_due(db, date, include_archived)?;
    let due_count = due.counts.due;

    let debts = outstanding_debts_as_of(db, date)?;
    let debts_due_count = debts.len() as u32;

    let severity = match (due_count > 0, debts_due_count > 0) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    };

    let now_t = now.time();
    let in_quiet = is_within_quiet_hours(now_t, qs_t, qe_t);

    let cadence = db.nag.config.cadence_minutes.max(1);

    let snoozed_until = db.nag.state.snoozed_until.clone();
    let last_sent_ts = db.nag.state.last_sent_ts.clone();

    // Suppression checks (quiet hours > snooze > cadence)
    let (should_send, next_check_at) = if in_quiet {
        (false, fmt_rfc3339(quiet_end_at(now, qs_t, qe_t)))
    } else if let Some(until) = snoozed_until.as_deref() {
        let until_dt = DateTime::parse_from_rfc3339(until)
            .map_err(|_| CliError::io("DB corrupted"))?;
        if now < until_dt {
            (false, until.trim().to_string())
        } else {
            // Snooze expired; continue evaluation.
            (false, String::new())
        }
    } else {
        (false, String::new())
    };

    let (should_send, next_check_at) = if !next_check_at.is_empty() {
        (should_send, next_check_at)
    } else if let Some(last) = last_sent_ts.as_deref() {
        let last_dt = DateTime::parse_from_rfc3339(last)
            .map_err(|_| CliError::io("DB corrupted"))?;
        let next = last_dt + Duration::minutes(cadence as i64);
        if now < next {
            (false, fmt_rfc3339(next))
        } else if severity > 0 {
            (true, now_ts.trim().to_string())
        } else {
            (false, fmt_rfc3339(now + Duration::minutes(cadence as i64)))
        }
    } else if severity > 0 {
        (true, now_ts.trim().to_string())
    } else {
        (false, fmt_rfc3339(now + Duration::minutes(cadence as i64)))
    };

    Ok(NagPlan {
        date: date.to_string(),
        now_ts: now_ts.trim().to_string(),
        quiet_start: db.nag.config.quiet_start.clone(),
        quiet_end: db.nag.config.quiet_end.clone(),
        cadence_minutes: cadence,
        snoozed_until,
        last_sent_ts,
        due_count,
        debts_due_count,
        severity,
        should_send,
        next_check_at,
    })
}

