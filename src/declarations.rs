use crate::date::parse_date_string;
use crate::error::CliError;
use crate::model::{Db, Declaration};
use crate::ts::validate_rfc3339;

pub fn next_declaration_id(db: &mut Db) -> String {
    let n = db.meta.next_declaration_number;
    let id = format!("d{:06}", n);
    db.meta.next_declaration_number = n + 1;
    id
}

pub fn declare(
    db: &mut Db,
    habit_id: &str,
    date: &str,
    ts: &str,
    text: &str,
) -> Result<Declaration, CliError> {
    parse_date_string(date, "date")?;
    validate_rfc3339(ts, "ts")?;

    let t = text.trim();
    if t.is_empty() {
        return Err(CliError::usage("Declaration text is required"));
    }

    let id = next_declaration_id(db);
    let decl = Declaration {
        id,
        habit_id: habit_id.to_string(),
        date: date.to_string(),
        ts: ts.trim().to_string(),
        text: t.to_string(),
    };
    db.declarations.push(decl.clone());
    Ok(decl)
}

pub fn has_declaration(db: &Db, habit_id: &str, date: &str) -> bool {
    db.declarations
        .iter()
        .any(|d| d.habit_id == habit_id && d.date == date)
}
