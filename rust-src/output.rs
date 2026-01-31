use crate::logic::{CheckinChange, ExportView, HabitStats, StatsView, StatusView};
use crate::model::{Habit, Period};
use chrono::Datelike;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn print_json<T: Serialize>(v: &T) -> Result<(), crate::logic::CliError> {
    let s = serde_json::to_string_pretty(v).map_err(crate::logic::CliError::io_other)?;
    println!("{}", s);
    Ok(())
}

pub fn print_added_habit(h: &Habit, _no_color: bool) -> Result<(), crate::logic::CliError> {
    println!("Created habit: {} {}", h.id.0, h.name);
    println!("Schedule: {}", schedule_to_string(h));
    println!("Target: {:?} {}", h.target.period, h.target.quantity);
    Ok(())
}

pub fn print_habit_list(habits: &[Habit], _no_color: bool) -> Result<(), crate::logic::CliError> {
    for h in habits {
        let arch = if h.archived { " (archived)" } else { "" };
        println!(
            "{}\t{}{}\t{}\t{:?}:{}, created {}",
            h.id.0,
            h.name,
            arch,
            schedule_to_string(h),
            h.target.period,
            h.target.quantity,
            h.created_date
        );
    }
    Ok(())
}

pub fn print_habit_show(h: &Habit, _no_color: bool) -> Result<(), crate::logic::CliError> {
    println!("{}\t{}", h.id.0, h.name);
    println!("schedule\t{}", schedule_to_string(h));
    println!("period\t{:?}", h.target.period);
    println!("target\t{}", h.target.quantity);
    println!("archived\t{}", h.archived);
    println!("created_date\t{}", h.created_date);
    println!(
        "archived_date\t{}",
        h.archived_date
            .map(|d| d.to_string())
            .unwrap_or_default()
    );
    if let Some(n) = &h.notes {
        println!("notes\t{}", n);
    }
    Ok(())
}

pub fn print_archive_result(h: &Habit, archived: bool) -> Result<(), crate::logic::CliError> {
    if archived {
        println!("Archived: {} ({})", h.name, h.id.0);
    } else {
        println!("Unarchived: {} ({})", h.name, h.id.0);
    }
    Ok(())
}

pub fn print_checkin_result(
    h: &Habit,
    date: chrono::NaiveDate,
    change: CheckinChange,
    total: u32,
) -> Result<(), crate::logic::CliError> {
    match change {
        CheckinChange::Add(q) => println!(
            "Checked in: {} ({}) on {} +{} (total {})",
            h.name, h.id.0, date, q, total
        ),
        CheckinChange::Set(q) => println!("Checked in: {} ({}) on {} ={}", h.name, h.id.0, date, q),
        CheckinChange::Delete => println!("Deleted check-in: {} ({}) on {}", h.name, h.id.0, date),
    }
    Ok(())
}

pub fn print_status_table(status: &StatusView, _no_color: bool) -> Result<(), crate::logic::CliError> {
    println!("Today ({})", status.date);
    for it in &status.today {
        let mark = if it.completed { "[x]" } else { "[ ]" };
        match it.period {
            Period::Day => {
                println!("- {} {:<18} {}/{}", mark, it.name, it.done, it.target);
            }
            Period::Week => {
                println!("- {} {:<18} {}/{} (weekly)", mark, it.name, it.done, it.target);
            }
        }
    }
    println!();

    let iso = status.iso_week_of.iso_week();
    println!("This week ({}-W{:02})", iso.year(), iso.week());
    for it in &status.week {
        if it.kind == "daily" {
            let x = it.completed_days.unwrap_or(0);
            let y = it.scheduled_days.unwrap_or(0);
            println!("- {:<18} {}/{} scheduled days done", it.name, x, y);
        } else {
            println!("- {:<18} {}/{} (weekly)", it.name, it.done, it.target);
        }
    }
    Ok(())
}

pub fn print_stats_table(stats: &StatsView, _no_color: bool) -> Result<(), crate::logic::CliError> {
    for it in &stats.items {
        print_stats_item(it);
    }
    Ok(())
}

fn print_stats_item(it: &HabitStats) {
    println!("{} ({})", it.name, it.habit_id);
    println!("  period: {:?}", it.period);
    println!("  window: {}..{}", it.window_from, it.window_to);
    println!("  current_streak: {}", it.current_streak);
    println!("  longest_streak: {}", it.longest_streak);
    println!("  success_rate: {:.3}", it.success_rate);
}

pub fn export_json(export: &ExportView, out: Option<&Path>) -> Result<(), crate::logic::CliError> {
    let s = serde_json::to_string_pretty(export).map_err(crate::logic::CliError::io_other)?;
    match out {
        None => {
            println!("{}", s);
            Ok(())
        }
        Some(p) => {
            fs::write(p, s).map_err(crate::logic::CliError::io)?;
            Ok(())
        }
    }
}

pub fn export_csv(export: &ExportView, out: Option<&Path>) -> Result<(), crate::logic::CliError> {
    let dir = out.ok_or_else(|| crate::logic::CliError::usage("CSV export requires --out <dir>"))?;
    fs::create_dir_all(dir).map_err(crate::logic::CliError::io)?;

    let habits_path = dir.join("habits.csv");
    let checkins_path = dir.join("checkins.csv");

    write_habits_csv(&habits_path, &export.habits)?;
    write_checkins_csv(&checkins_path, &export.checkins)?;

    println!("Wrote {} and {}", habits_path.display(), checkins_path.display());
    Ok(())
}

fn write_habits_csv(path: &PathBuf, habits: &[Habit]) -> Result<(), crate::logic::CliError> {
    let mut out = String::new();
    out.push_str("id,name,schedule,period,target,notes,archived,created_date,archived_date\n");
    for h in habits {
        out.push_str(&csv_row(&[
            &h.id.0,
            &h.name,
            &schedule_to_string(h),
            &format!("{:?}", h.target.period).to_lowercase(),
            &h.target.quantity.to_string(),
            &h.notes.clone().unwrap_or_default(),
            &h.archived.to_string(),
            &h.created_date.to_string(),
            &h.archived_date.map(|d| d.to_string()).unwrap_or_default(),
        ]));
        out.push('\n');
    }
    fs::write(path, out).map_err(crate::logic::CliError::io)
}

fn write_checkins_csv(path: &PathBuf, checkins: &[crate::model::Checkin]) -> Result<(), crate::logic::CliError> {
    let mut out = String::new();
    out.push_str("habit_id,date,quantity\n");
    for c in checkins {
        out.push_str(&csv_row(&[&c.habit_id.0, &c.date.to_string(), &c.quantity.to_string()]));
        out.push('\n');
    }
    fs::write(path, out).map_err(crate::logic::CliError::io)
}

fn csv_row(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|f| {
            if f.contains(',') || f.contains('"') || f.contains('\n') {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                (*f).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn schedule_to_string(h: &Habit) -> String {
    let days = h.schedule.days();
    if days == [1, 2, 3, 4, 5, 6, 7] {
        return "everyday".to_string();
    }
    if days == [1, 2, 3, 4, 5] {
        return "weekdays".to_string();
    }
    if days == [6, 7] {
        return "weekends".to_string();
    }
    let names: Vec<&str> = days
        .iter()
        .map(|d| match d {
            1 => "mon",
            2 => "tue",
            3 => "wed",
            4 => "thu",
            5 => "fri",
            6 => "sat",
            7 => "sun",
            _ => "?",
        })
        .collect();
    names.join(",")
}
