mod checkins;
mod date;
mod db;
mod error;
mod export;
mod habits;
mod model;
mod output;
mod schedule;
mod stable_json;
mod stats;
mod status;

use crate::checkins::{
    add_quantity, list_checkins_for_habit, list_checkins_in_range, set_quantity,
};
use crate::date::{add_days, iso_week_start, parse_date_string, system_today_utc};
use crate::db::{read_db, resolve_db_path, update_db};
use crate::error::CliError;
use crate::export::export_csv_to_dir;
use crate::habits::{
    list_habits, make_habit, next_habit_id, select_habit_index, stable_habit_sort,
};
use crate::output::{render_simple_table, Styler};
use crate::schedule::schedule_to_string;
use crate::stable_json::stable_to_string_pretty;
use crate::stats::build_stats;
use crate::status::build_status;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::collections::BTreeSet;
use std::fs;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Format {
    Table,
    Json,
    Csv,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Period {
    Day,
    Week,
}

impl Period {
    fn as_str(&self) -> &'static str {
        match self {
            Period::Day => "day",
            Period::Week => "week",
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "habit", version, about = "Local habit tracking CLI")]
struct Cli {
    /// Overrides the DB path for this invocation.
    #[arg(long, global = true)]
    db: Option<String>,

    /// Overrides logical "today" for deterministic output/testing.
    #[arg(long, global = true)]
    today: Option<String>,

    /// Output format. Most commands support table/json. `export` supports json/csv.
    #[arg(long, global = true, value_enum, default_value = "table")]
    format: Format,

    /// Disables ANSI color output.
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Add(AddArgs),
    List(ListArgs),
    Show(SelectorArgs),
    Archive(SelectorArgs),
    Unarchive(SelectorArgs),
    Checkin(CheckinArgs),
    Status(StatusArgs),
    Stats(StatsArgs),
    Export(ExportArgs),
}

#[derive(Args, Debug)]
struct AddArgs {
    name: String,

    /// One of: everyday, weekdays, weekends, mon,tue,...,sun
    #[arg(long, default_value = "everyday")]
    schedule: String,

    #[arg(long, value_enum, default_value = "day")]
    period: Period,

    /// Integer >= 1
    #[arg(long, default_value_t = 1)]
    target: u32,

    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args, Debug)]
struct ListArgs {
    /// Include archived habits
    #[arg(long)]
    all: bool,
}

#[derive(Args, Debug)]
struct SelectorArgs {
    /// Habit selector: exact id (h0001) or unique name prefix (case-insensitive)
    habit: String,
}

#[derive(Args, Debug)]
struct CheckinArgs {
    /// Habit selector: exact id (h0001) or unique name prefix (case-insensitive)
    habit: String,

    #[arg(long)]
    date: Option<String>,

    /// Integer >= 1 (default 1)
    #[arg(long)]
    qty: Option<u32>,

    /// Integer >= 0 (sets the aggregate quantity for that date)
    #[arg(long)]
    set: Option<u32>,

    /// Deletes the check-in record for that date
    #[arg(long)]
    delete: bool,
}

#[derive(Args, Debug)]
struct StatusArgs {
    /// The "today" shown in the Today section
    #[arg(long)]
    date: Option<String>,

    /// Choose which week to show (defaults to week containing today)
    #[arg(long = "week-of")]
    week_of: Option<String>,

    #[arg(long)]
    include_archived: bool,
}

#[derive(Args, Debug)]
struct StatsArgs {
    /// Optional habit selector
    habit: Option<String>,

    #[arg(long)]
    from: Option<String>,

    #[arg(long)]
    to: Option<String>,
}

#[derive(Args, Debug)]
struct ExportArgs {
    #[arg(long)]
    out: Option<String>,

    #[arg(long)]
    from: Option<String>,

    #[arg(long)]
    to: Option<String>,

    #[arg(long)]
    include_archived: bool,
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(2);
        }
    };

    let exit = match run(cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{}", e);
            e.exit_code
        }
    };

    std::process::exit(exit);
}

fn print_line(s: &str) {
    println!("{}", s);
}

fn print_json<T: serde::Serialize>(obj: &T) -> Result<(), CliError> {
    let s = stable_to_string_pretty(obj).map_err(|_| CliError::io("DB IO error"))?;
    println!("{}", s);
    Ok(())
}

fn resolve_today(cli_today: Option<&str>) -> Result<String, CliError> {
    if let Some(t) = cli_today {
        parse_date_string(t, "today")?;
        return Ok(t.to_string());
    }

    if let Ok(t) = std::env::var("HABITCLI_TODAY") {
        let tt = t.trim();
        if !tt.is_empty() {
            parse_date_string(tt, "today")?;
            return Ok(tt.to_string());
        }
    }

    Ok(system_today_utc())
}

fn resolve_color_enabled(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    true
}

fn ensure_format_supported(format: Format, allow_csv: bool) -> Result<(), CliError> {
    if format == Format::Csv && !allow_csv {
        return Err(CliError::usage(
            "--format csv is only supported by `habit export`",
        ));
    }
    Ok(())
}

fn run(cli: Cli) -> Result<(), CliError> {
    let db_path = resolve_db_path(cli.db.as_deref())?;
    let today = resolve_today(cli.today.as_deref())?;

    let styler = Styler::new(resolve_color_enabled(cli.no_color));

    match cli.command {
        Command::Add(args) => {
            ensure_format_supported(cli.format, false)?;

            let created = update_db(&db_path, |db| {
                let id = next_habit_id(db);
                let habit = make_habit(
                    id,
                    &args.name,
                    &args.schedule,
                    args.period.as_str(),
                    args.target,
                    args.notes.as_deref(),
                    &today,
                )?;
                db.habits.push(habit.clone());
                Ok(habit)
            })?;

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct Out {
                    habit: crate::model::Habit,
                }
                print_json(&Out { habit: created })?;
            } else {
                let row = vec![
                    created.id.clone(),
                    created.name.clone(),
                    schedule_to_string(&created.schedule),
                    format!("{}/{}", created.target.quantity, created.target.period),
                ];
                print_line(&render_simple_table(
                    &["id", "name", "schedule", "target"],
                    &[row],
                ));
            }

            Ok(())
        }

        Command::List(args) => {
            ensure_format_supported(cli.format, false)?;

            let db = read_db(&db_path)?;
            let habits = list_habits(&db, args.all);

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct Out {
                    habits: Vec<crate::model::Habit>,
                }
                print_json(&Out { habits })?;
            } else {
                let rows: Vec<Vec<String>> = habits
                    .iter()
                    .map(|h| {
                        vec![
                            h.id.clone(),
                            h.name.clone(),
                            schedule_to_string(&h.schedule),
                            format!("{}/{}", h.target.quantity, h.target.period),
                            if h.archived {
                                "yes".to_string()
                            } else {
                                "no".to_string()
                            },
                        ]
                    })
                    .collect();

                print_line(&render_simple_table(
                    &["id", "name", "schedule", "target", "archived"],
                    &rows,
                ));
            }

            Ok(())
        }

        Command::Show(args) => {
            ensure_format_supported(cli.format, false)?;

            let db = read_db(&db_path)?;
            let idx = select_habit_index(&db, &args.habit, true)?;
            let habit = db.habits[idx].clone();
            let checkins = list_checkins_for_habit(&db, &habit.id);

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct Out {
                    habit: crate::model::Habit,
                    checkins: Vec<crate::model::Checkin>,
                }
                print_json(&Out { habit, checkins })?;
            } else {
                print_line(&format!("{} ({})", habit.name, habit.id));
                print_line(&format!(
                    "schedule: {}",
                    schedule_to_string(&habit.schedule)
                ));
                print_line(&format!(
                    "target: {}/{}",
                    habit.target.quantity, habit.target.period
                ));
                print_line(&format!(
                    "archived: {}",
                    if habit.archived { "yes" } else { "no" }
                ));
                print_line(&format!("created_date: {}", habit.created_date));
                if let Some(ad) = habit.archived_date.as_deref() {
                    print_line(&format!("archived_date: {}", ad));
                }
                if let Some(n) = habit.notes.as_deref() {
                    print_line(&format!("notes: {}", n));
                }
                if !checkins.is_empty() {
                    print_line("checkins:");
                    for c in checkins.iter() {
                        print_line(&format!("- {} {}", c.date, c.quantity));
                    }
                }
            }

            Ok(())
        }

        Command::Archive(args) => {
            ensure_format_supported(cli.format, false)?;

            let updated = update_db(&db_path, |db| {
                let idx = select_habit_index(db, &args.habit, true)?;
                let habit = &mut db.habits[idx];

                habit.archived = true;
                if habit.archived_date.is_none() {
                    habit.archived_date = Some(today.clone());
                }

                Ok(habit.clone())
            })?;

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct Out {
                    habit: crate::model::Habit,
                }
                print_json(&Out { habit: updated })?;
            } else {
                print_line(&format!("Archived: {} ({})", updated.name, updated.id));
            }

            Ok(())
        }

        Command::Unarchive(args) => {
            ensure_format_supported(cli.format, false)?;

            let updated = update_db(&db_path, |db| {
                let idx = select_habit_index(db, &args.habit, true)?;
                let habit = &mut db.habits[idx];

                habit.archived = false;
                habit.archived_date = None;

                Ok(habit.clone())
            })?;

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct Out {
                    habit: crate::model::Habit,
                }
                print_json(&Out { habit: updated })?;
            } else {
                print_line(&format!("Unarchived: {} ({})", updated.name, updated.id));
            }

            Ok(())
        }

        Command::Checkin(args) => {
            ensure_format_supported(cli.format, false)?;

            let date = args.date.as_deref().unwrap_or(&today);
            parse_date_string(date, "date")?;

            if args.delete && (args.qty.is_some() || args.set.is_some()) {
                return Err(CliError::usage(
                    "Invalid flags: --delete conflicts with --qty/--set",
                ));
            }
            if args.qty.is_some() && args.set.is_some() {
                return Err(CliError::usage("Invalid flags: --qty conflicts with --set"));
            }

            let qty = args.qty.unwrap_or(1);
            let set = args.set.unwrap_or(0);

            #[derive(Debug)]
            struct ResultRow {
                habit_id: String,
                habit_name: String,
                date: String,
                action: String,
                delta: Option<u32>,
                quantity: u32,
            }

            let result = update_db(&db_path, |db| {
                let idx = select_habit_index(db, &args.habit, true)?;
                let habit = db.habits[idx].clone();

                if args.delete {
                    set_quantity(db, &habit.id, date, 0)?;
                    return Ok(ResultRow {
                        habit_id: habit.id,
                        habit_name: habit.name,
                        date: date.to_string(),
                        action: "delete".to_string(),
                        delta: None,
                        quantity: 0,
                    });
                }

                if args.set.is_some() {
                    set_quantity(db, &habit.id, date, set)?;
                    return Ok(ResultRow {
                        habit_id: habit.id,
                        habit_name: habit.name,
                        date: date.to_string(),
                        action: "set".to_string(),
                        delta: None,
                        quantity: set,
                    });
                }

                let total = add_quantity(db, &habit.id, date, qty)?;
                Ok(ResultRow {
                    habit_id: habit.id,
                    habit_name: habit.name,
                    date: date.to_string(),
                    action: "add".to_string(),
                    delta: Some(qty),
                    quantity: total,
                })
            })?;

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct HabitMini {
                    id: String,
                    name: String,
                }

                #[derive(serde::Serialize)]
                struct Out {
                    habit: HabitMini,
                    date: String,
                    action: String,
                    quantity: u32,
                    delta: Option<u32>,
                }

                print_json(&Out {
                    habit: HabitMini {
                        id: result.habit_id,
                        name: result.habit_name,
                    },
                    date: result.date,
                    action: result.action,
                    quantity: result.quantity,
                    delta: result.delta,
                })?;
            } else if result.action == "delete" {
                print_line(&format!(
                    "Deleted check-in: {} ({}) on {}",
                    result.habit_name, result.habit_id, result.date
                ));
            } else if result.action == "set" {
                print_line(&format!(
                    "Set check-in: {} ({}) on {} ={}",
                    result.habit_name, result.habit_id, result.date, result.quantity
                ));
            } else {
                print_line(&format!(
                    "Checked in: {} ({}) on {} +{} (total {})",
                    result.habit_name,
                    result.habit_id,
                    result.date,
                    result.delta.unwrap_or(0),
                    result.quantity
                ));
            }

            Ok(())
        }

        Command::Status(args) => {
            ensure_format_supported(cli.format, false)?;

            let date = args.date.as_deref().unwrap_or(&today);
            parse_date_string(date, "date")?;

            if let Some(wo) = args.week_of.as_deref() {
                parse_date_string(wo, "week-of")?;
            }

            let db = read_db(&db_path)?;
            let data = build_status(&db, date, args.week_of.as_deref(), args.include_archived)?;

            if cli.format == Format::Json {
                print_json(&data)?;
            } else {
                print_line(&format!("Today ({})", data.today.date));
                if data.today.habits.is_empty() {
                    print_line(&styler.gray("(no scheduled habits)"));
                } else {
                    for h in data.today.habits.iter() {
                        let mark = if h.done {
                            styler.green("[x]")
                        } else {
                            "[ ]".to_string()
                        };
                        let progress = if h.period == "day" {
                            format!("{}/{}", h.quantity, h.target)
                        } else {
                            format!("{}/{} (weekly)", h.quantity, h.target)
                        };
                        print_line(&format!("- {} {} {}", mark, h.name, progress));
                    }
                }

                print_line("");
                print_line(&format!("This week ({})", data.week.id));
                for h in data.week.habits.iter() {
                    match h {
                        crate::status::WeekHabitRow::Day(r) => {
                            print_line(&format!(
                                "- {} {}/{} scheduled days done",
                                r.name, r.done_scheduled_days, r.scheduled_days
                            ));
                        }
                        crate::status::WeekHabitRow::Week(r) => {
                            print_line(&format!(
                                "- {} {}/{} (weekly)",
                                r.name, r.quantity, r.target
                            ));
                        }
                    }
                }
            }

            Ok(())
        }

        Command::Stats(args) => {
            ensure_format_supported(cli.format, false)?;

            let db = read_db(&db_path)?;

            let habits: Vec<crate::model::Habit> = if let Some(sel) = args.habit.as_deref() {
                let idx = select_habit_index(&db, sel, true)?;
                vec![db.habits[idx].clone()]
            } else {
                db.habits.iter().filter(|h| !h.archived).cloned().collect()
            };

            let mut habits_sorted = habits;
            habits_sorted.sort_by(stable_habit_sort);

            let to_eff = args.to.unwrap_or_else(|| today.clone());
            parse_date_string(&to_eff, "to")?;

            if let Some(ref f) = args.from {
                parse_date_string(f, "from")?;
                if f > &to_eff {
                    return Err(CliError::usage("Invalid range: from > to"));
                }
            }

            let rows: Vec<crate::stats::StatsRow> = if let Some(from_eff) = args.from.as_deref() {
                build_stats(&db, &habits_sorted, from_eff, &to_eff)?
            } else {
                // Per-habit default windows.
                let mut out: Vec<crate::stats::StatsRow> = Vec::new();
                for h in habits_sorted.iter() {
                    if h.target.period == "week" {
                        let end_week = iso_week_start(&to_eff)?;
                        let from2 = add_days(&end_week, -7 * (12 - 1))?;
                        let to2 = add_days(&end_week, 6)?;
                        let mut one = build_stats(&db, std::slice::from_ref(h), &from2, &to2)?;
                        if let Some(r) = one.pop() {
                            out.push(r);
                        }
                    } else {
                        let from2 = add_days(&to_eff, -29)?;
                        let mut one = build_stats(&db, std::slice::from_ref(h), &from2, &to_eff)?;
                        if let Some(r) = one.pop() {
                            out.push(r);
                        }
                    }
                }
                out
            };

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct Out {
                    stats: Vec<crate::stats::StatsRow>,
                }
                print_json(&Out { stats: rows })?;
            } else {
                let mut table_rows: Vec<Vec<String>> = Vec::new();
                for r in rows.iter() {
                    let rate = if r.success_rate.eligible == 0 {
                        "n/a".to_string()
                    } else {
                        let pct = (r.success_rate.rate.unwrap_or(0.0) * 100.0).round() as i64;
                        format!("{}%", pct)
                    };

                    table_rows.push(vec![
                        r.habit_id.clone(),
                        r.name.clone(),
                        r.period.clone(),
                        r.current_streak.to_string(),
                        r.longest_streak.to_string(),
                        format!(
                            "{} ({}/{})",
                            rate, r.success_rate.successes, r.success_rate.eligible
                        ),
                    ]);
                }

                print_line(&render_simple_table(
                    &["id", "name", "period", "current", "longest", "success"],
                    &table_rows,
                ));
            }

            Ok(())
        }

        Command::Export(args) => {
            // `export` supports json/csv; `table` is invalid.
            if cli.format == Format::Table {
                return Err(CliError::usage("`habit export` requires --format json|csv"));
            }

            let from = args.from.as_deref();
            let to = args.to.as_deref();

            if let Some(f) = from {
                parse_date_string(f, "from")?;
            }
            if let Some(t) = to {
                parse_date_string(t, "to")?;
            }
            if let (Some(f), Some(t)) = (from, to) {
                if f > t {
                    return Err(CliError::usage("Invalid range: from > to"));
                }
            }

            let db = read_db(&db_path)?;
            let habits = list_habits(&db, args.include_archived);
            let habit_ids: BTreeSet<String> = habits.iter().map(|h| h.id.clone()).collect();
            let checkins = list_checkins_in_range(&db, from, to, Some(&habit_ids));

            if cli.format == Format::Json {
                #[derive(serde::Serialize)]
                struct Payload {
                    version: u32,
                    habits: Vec<crate::model::Habit>,
                    checkins: Vec<crate::model::Checkin>,
                }

                let payload = Payload {
                    version: 1,
                    habits,
                    checkins,
                };
                let data = stable_to_string_pretty(&payload)
                    .map_err(|_| CliError::io("DB IO error"))?
                    + "\n";

                if let Some(p) = args.out.as_deref() {
                    fs::write(p, data.as_bytes()).map_err(|_| CliError::io("DB IO error"))?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o600));
                    }
                } else {
                    print!("{}", data);
                }
            } else {
                let out_dir = args
                    .out
                    .as_deref()
                    .ok_or_else(|| CliError::usage("CSV export requires --out <dir>"))?;
                export_csv_to_dir(out_dir, &habits, &checkins)?;
            }

            Ok(())
        }
    }
}
