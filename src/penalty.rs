use crate::completion::{counted_quantity, is_declared};
use crate::date::{add_days, parse_date_string};
use crate::error::CliError;
use crate::excuses::has_allowed_excuse;
use crate::habits::is_scheduled_on;
use crate::model::{Db, PenaltyAction, PenaltyActionKind, PenaltyDebt, PenaltyRule};
use crate::ts::validate_rfc3339;
use std::collections::{BTreeMap, BTreeSet};

fn compact_date(date: &str) -> String {
    date.replace('-', "")
}

pub fn next_penalty_rule_id(db: &mut Db) -> String {
    let n = db.meta.next_penalty_rule_number;
    let id = format!("pr{:06}", n);
    db.meta.next_penalty_rule_number = n + 1;
    id
}

pub fn upsert_rule(
    db: &mut Db,
    habit_id: &str,
    armed_date: &str,
    armed_ts: &str,
    multiplier: u32,
    cap: u32,
    deadline_days: u32,
) -> Result<PenaltyRule, CliError> {
    parse_date_string(armed_date, "date")?;
    validate_rfc3339(armed_ts, "ts")?;

    if multiplier < 1 {
        return Err(CliError::usage("Invalid multiplier"));
    }
    if cap < 1 {
        return Err(CliError::usage("Invalid cap"));
    }

    if let Some(i) = db.penalty_rules.iter().position(|r| r.habit_id == habit_id) {
        db.penalty_rules[i].multiplier = multiplier;
        db.penalty_rules[i].cap = cap;
        db.penalty_rules[i].deadline_days = deadline_days;
        db.penalty_rules[i].armed_date = armed_date.to_string();
        db.penalty_rules[i].armed_ts = armed_ts.trim().to_string();
        return Ok(db.penalty_rules[i].clone());
    }

    let rule = PenaltyRule {
        id: next_penalty_rule_id(db),
        habit_id: habit_id.to_string(),
        multiplier,
        cap,
        deadline_days,
        armed_date: armed_date.to_string(),
        armed_ts: armed_ts.trim().to_string(),
    };
    db.penalty_rules.push(rule.clone());
    Ok(rule)
}

pub fn debt_id_for(habit_id: &str, trigger_date: &str) -> String {
    format!("pd_{}_{}", habit_id, compact_date(trigger_date))
}

fn action_id_for(debt_id: &str, kind: PenaltyActionKind) -> String {
    let k = match kind {
        PenaltyActionKind::Resolve => "resolve",
        PenaltyActionKind::Void => "void",
    };
    format!("pa_{}_{}", debt_id, k)
}

pub fn debt_closed_map(db: &Db) -> BTreeSet<String> {
    let mut closed: BTreeSet<String> = BTreeSet::new();
    for a in db.penalty_actions.iter() {
        closed.insert(a.debt_id.clone());
    }
    closed
}

pub fn outstanding_debts_as_of(db: &Db, date: &str) -> Result<Vec<PenaltyDebt>, CliError> {
    parse_date_string(date, "date")?;
    let closed = debt_closed_map(db);

    let mut out: Vec<PenaltyDebt> = db
        .penalty_debts
        .iter()
        .filter(|d| !closed.contains(&d.id))
        .filter(|d| d.due_date.as_str() <= date)
        .cloned()
        .collect();

    out.sort_by(|a, b| {
        if a.due_date != b.due_date {
            a.due_date.cmp(&b.due_date)
        } else if a.habit_id != b.habit_id {
            a.habit_id.cmp(&b.habit_id)
        } else {
            a.id.cmp(&b.id)
        }
    });

    Ok(out)
}

fn rule_map(db: &Db) -> BTreeMap<String, PenaltyRule> {
    let mut m = BTreeMap::new();
    for r in db.penalty_rules.iter() {
        m.insert(r.habit_id.clone(), r.clone());
    }
    m
}

pub fn tick(
    db: &mut Db,
    date: &str,
    ts: &str,
    include_archived: bool,
) -> Result<Vec<PenaltyDebt>, CliError> {
    parse_date_string(date, "date")?;
    validate_rfc3339(ts, "ts")?;

    let rules = rule_map(db);
    let closed = debt_closed_map(db);

    let mut created: Vec<PenaltyDebt> = Vec::new();

    for h in db.habits.iter() {
        if !include_archived && h.archived {
            continue;
        }
        if h.target.period != "day" {
            // MVP: penalty tick only evaluates day-period habits.
            continue;
        }

        let rule = match rules.get(&h.id) {
            Some(r) => r.clone(),
            None => continue,
        };

        if !is_scheduled_on(h, date)? {
            continue;
        }

        if has_allowed_excuse(db, &h.id, date) {
            continue;
        }

        let done_qty = counted_quantity(db, h, date);
        let declared = is_declared(db, h, date);
        let habit_done = declared && done_qty >= h.target.quantity;

        // If there is outstanding debt due today and it's not resolved/voided, treat it as a miss.
        let outstanding_due_today: Option<PenaltyDebt> = db
            .penalty_debts
            .iter()
            .filter(|d| d.habit_id == h.id)
            .filter(|d| d.due_date == date)
            .filter(|d| !closed.contains(&d.id))
            .cloned()
            .max_by(|a, b| a.quantity.cmp(&b.quantity));

        let missed_debt = outstanding_due_today.is_some();
        if habit_done && !missed_debt {
            continue;
        }

        let debt_id = debt_id_for(&h.id, date);
        if db.penalty_debts.iter().any(|d| d.id == debt_id) {
            // idempotent
            continue;
        }

        let base_qty = match outstanding_due_today {
            Some(d) => d.quantity.max(h.target.quantity),
            None => h.target.quantity,
        };

        let mut qty = base_qty.saturating_mul(rule.multiplier);
        qty = qty.min(rule.cap);

        let due_date = add_days(date, 1)?;
        let debt = PenaltyDebt {
            id: debt_id,
            habit_id: h.id.clone(),
            trigger_date: date.to_string(),
            due_date,
            quantity: qty,
            rule_id: rule.id.clone(),
            created_date: date.to_string(),
            created_ts: ts.trim().to_string(),
        };

        db.penalty_debts.push(debt.clone());
        created.push(debt);
    }

    // Stable sort output.
    created.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(created)
}

pub fn resolve_or_void(
    db: &mut Db,
    debt_id: &str,
    kind: PenaltyActionKind,
    date: &str,
    ts: &str,
    reason: &str,
) -> Result<PenaltyAction, CliError> {
    parse_date_string(date, "date")?;
    validate_rfc3339(ts, "ts")?;

    let r = reason.trim();
    if r.is_empty() {
        return Err(CliError::usage("Reason is required"));
    }

    if !db.penalty_debts.iter().any(|d| d.id == debt_id) {
        return Err(CliError::not_found(format!(
            "Penalty debt not found: {}",
            debt_id
        )));
    }

    let action_id = action_id_for(debt_id, kind);
    if let Some(a) = db.penalty_actions.iter().find(|a| a.id == action_id) {
        return Ok(a.clone());
    }

    // If any action already exists for this debt, treat as idempotent (return the first).
    if let Some(a) = db.penalty_actions.iter().find(|a| a.debt_id == debt_id) {
        return Ok(a.clone());
    }

    let action = PenaltyAction {
        id: action_id,
        debt_id: debt_id.to_string(),
        kind,
        date: date.to_string(),
        ts: ts.trim().to_string(),
        reason: r.to_string(),
    };
    db.penalty_actions.push(action.clone());
    Ok(action)
}
