use crate::error::CliError;
use crate::model::{Checkin, Habit};
use crate::schedule::schedule_to_string;
use std::fs;
use std::io::Write;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn csv_escape(value: &str) -> String {
    if value.contains(['\n', '\r', '"', ',']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn to_csv_line(values: &[String]) -> String {
    values.iter().map(|v| csv_escape(v)).collect::<Vec<String>>().join(",")
}

pub fn export_csv_to_dir(out_dir: &str, habits: &[Habit], checkins: &[Checkin]) -> Result<(), CliError> {
    let out_path = Path::new(out_dir);
    fs::create_dir_all(out_path).map_err(|_| CliError::io("DB IO error"))?;

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(out_path, fs::Permissions::from_mode(0o700));
    }

    let habits_header: Vec<String> = vec![
        "id",
        "name",
        "schedule",
        "period",
        "target",
        "notes",
        "archived",
        "created_date",
        "archived_date",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let mut habit_lines: Vec<String> = Vec::new();
    habit_lines.push(to_csv_line(&habits_header));
    for h in habits.iter() {
        habit_lines.push(to_csv_line(&[
            h.id.clone(),
            h.name.clone(),
            schedule_to_string(&h.schedule),
            h.target.period.clone(),
            h.target.quantity.to_string(),
            h.notes.clone().unwrap_or_default(),
            if h.archived { "true".to_string() } else { "false".to_string() },
            h.created_date.clone(),
            h.archived_date.clone().unwrap_or_default(),
        ]));
    }

    let checkins_header: Vec<String> = vec!["habit_id", "date", "quantity"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let mut checkin_lines: Vec<String> = Vec::new();
    checkin_lines.push(to_csv_line(&checkins_header));
    for c in checkins.iter() {
        checkin_lines.push(to_csv_line(&[
            c.habit_id.clone(),
            c.date.clone(),
            c.quantity.to_string(),
        ]));
    }

    let habits_csv = out_path.join("habits.csv");
    let checkins_csv = out_path.join("checkins.csv");

    {
        let mut f = fs::File::create(&habits_csv).map_err(|_| CliError::io("DB IO error"))?;
        #[cfg(unix)]
        {
            let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
        }
        f.write_all(habit_lines.join("\n").as_bytes())
            .map_err(|_| CliError::io("DB IO error"))?;
        let _ = f.write_all(b"\n");
    }

    {
        let mut f = fs::File::create(&checkins_csv).map_err(|_| CliError::io("DB IO error"))?;
        #[cfg(unix)]
        {
            let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
        }
        f.write_all(checkin_lines.join("\n").as_bytes())
            .map_err(|_| CliError::io("DB IO error"))?;
        let _ = f.write_all(b"\n");
    }

    Ok(())
}
