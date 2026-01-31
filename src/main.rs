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

use crate::checkins::{add_quantity, list_checkins_for_habit, list_checkins_in_range, set_quantity};
use crate::date::{add_days, iso_week_start, parse_date_string, system_today_utc};
use crate::db::{read_db, resolve_db_path, update_db};
use crate::error::CliError;
use crate::export::export_csv_to_dir;
use crate::habits::{list_habits, make_habit, next_habit_id, select_habit_index};
use crate::output::{render_simple_table, Styler};
use crate::schedule::schedule_to_string;
use crate::stable_json::stable_to_string_pretty;
use crate::stats::build_stats;
use crate::status::build_status;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const COMMANDS: [&str; 9] = [
    "add",
    "list",
    "show",
    "archive",
    "unarchive",
    "checkin",
    "status",
    "stats",
    "export",
];

#[derive(Debug, Default, Clone)]
struct GlobalOpts {
    db: Option<String>,
    today: Option<String>,
    format: Option<String>,
    no_color: bool,
    help: bool,
}

#[derive(Debug, Clone)]
enum OptValue {
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone)]
struct CommandParse {
    opts: BTreeMap<String, OptValue>,
    positionals: Vec<String>,
}

#[derive(Debug, Clone)]
enum ArgSpec {
    Bool,
    Enum(&'static [&'static str]),
    Value,
}

fn usage() -> String {
    [
        "habit — local habit tracking CLI",
        "",
        "Usage:",
        "  habit [global options] <command> [options]",
        "",
        "Commands:",
        "  add, list, show, archive, unarchive, checkin, status, stats, export",
        "",
        "Global options:",
        "  --db <path>",
        "  --today <YYYY-MM-DD>",
        "  --format table|json",
        "  --no-color",
        "  --help",
        "",
    ]
    .join("\n")
}

fn print_line(s: &str) {
    print!("{}\n", s);
}

fn print_err_line(s: &str) {
    eprintln!("{}", s);
}

fn print_json<T: serde::Serialize>(obj: &T) -> Result<(), CliError> {
    let s = stable_to_string_pretty(obj).map_err(|_| CliError::io("DB IO error"))?;
    print!("{}\n", s);
    Ok(())
}

fn pick_command_index(argv: &[String]) -> Option<usize> {
    argv.iter()
        .position(|a| COMMANDS.iter().any(|c| c == a.as_str()))
}

fn parse_global_opts_from_args(args: &[String], allow_format: bool) -> Result<(GlobalOpts, Vec<String>), CliError> {
    let mut opts = GlobalOpts::default();
    let mut rest: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--help" || a == "-h" {
            opts.help = true;
            i += 1;
            continue;
        }
        if a == "--no-color" {
            opts.no_color = true;
            i += 1;
            continue;
        }
        if a == "--db" {
            i += 1;
            let v = args.get(i).ok_or_else(|| CliError::usage("Missing value for --db"))?;
            opts.db = Some(v.clone());
            i += 1;
            continue;
        }
        if a == "--today" {
            i += 1;
            let v = args.get(i).ok_or_else(|| CliError::usage("Missing value for --today"))?;
            opts.today = Some(v.clone());
            i += 1;
            continue;
        }
        if allow_format && a == "--format" {
            i += 1;
            let v = args.get(i).ok_or_else(|| CliError::usage("Missing value for --format"))?;
            if v != "table" && v != "json" {
                return Err(CliError::usage(format!("Invalid format: {}", v)));
            }
            opts.format = Some(v.clone());
            i += 1;
            continue;
        }

        rest.push(args[i].clone());
        i += 1;
    }

    Ok((opts, rest))
}

fn parse_command_opts(args: &[String], allowed: &BTreeMap<&'static str, ArgSpec>) -> Result<CommandParse, CliError> {
    let mut opts: BTreeMap<String, OptValue> = BTreeMap::new();
    let mut positionals: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();

        if a == "--help" || a == "-h" {
            opts.insert("help".to_string(), OptValue::Bool(true));
            i += 1;
            continue;
        }

        if a.starts_with("--") {
            let key = &a[2..];
            let spec = allowed
                .get(key)
                .ok_or_else(|| CliError::usage(format!("Unknown option: {}", a)))?;

            match spec {
                ArgSpec::Bool => {
                    opts.insert(key.to_string(), OptValue::Bool(true));
                    i += 1;
                }
                ArgSpec::Enum(values) => {
                    i += 1;
                    let v = args
                        .get(i)
                        .ok_or_else(|| CliError::usage(format!("Missing value for {}", a)))?;
                    if !values.iter().any(|vv| vv == v) {
                        return Err(CliError::usage(format!("Invalid {}: {}", key, v)));
                    }
                    opts.insert(key.to_string(), OptValue::Str(v.clone()));
                    i += 1;
                }
                ArgSpec::Value => {
                    i += 1;
                    let v = args
                        .get(i)
                        .ok_or_else(|| CliError::usage(format!("Missing value for {}", a)))?;
                    opts.insert(key.to_string(), OptValue::Str(v.clone()));
                    i += 1;
                }
            }
            continue;
        }

        positionals.push(args[i].clone());
        i += 1;
    }

    Ok(CommandParse { opts, positionals })
}

fn opt_bool(parsed: &CommandParse, key: &str) -> bool {
    matches!(parsed.opts.get(key), Some(OptValue::Bool(true)))
}

fn opt_str<'a>(parsed: &'a CommandParse, key: &str) -> Option<&'a str> {
    match parsed.opts.get(key) {
        Some(OptValue::Str(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn resolve_today(cli_today: Option<&str>) -> Result<String, CliError> {
    let by_arg = cli_today
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let by_env = std::env::var("HABITCLI_TODAY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let today = by_arg.or(by_env).unwrap_or_else(system_today_utc);
    parse_date_string(&today, "today")?;
    Ok(today)
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

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let exit = match run_cli(&argv) {
        Ok(()) => 0,
        Err(e) => {
            let line = e
                .message
                .split(['\n', '\r'])
                .next()
                .unwrap_or("DB IO error");
            print_err_line(line);
            e.exit_code
        }
    };
    std::process::exit(exit);
}

fn run_cli(argv: &[String]) -> Result<(), CliError> {
    let cmd_index = pick_command_index(argv);
    let has_help = argv.iter().any(|a| a == "--help" || a == "-h");

    if cmd_index.is_none() {
        if has_help || argv.is_empty() {
            print_line(&usage());
            return Ok(());
        }
        return Err(CliError::usage("Missing command"));
    }

    let cmd_index = cmd_index.unwrap();
    let cmd = argv[cmd_index].clone();

    let pre = &argv[0..cmd_index];
    let post = &argv[(cmd_index + 1)..];

    let (pre_opts, pre_rest) = parse_global_opts_from_args(pre, true)?;
    if !pre_rest.is_empty() {
        return Err(CliError::usage("Invalid arguments"));
    }

    // after the subcommand, we avoid capturing --format to keep `habit export --format ...` unambiguous.
    let (post_opts, cmd_args) = parse_global_opts_from_args(post, false)?;

    let global_opts = GlobalOpts {
        db: post_opts.db.or(pre_opts.db),
        today: post_opts.today.or(pre_opts.today),
        format: post_opts.format.or(pre_opts.format),
        no_color: pre_opts.no_color || post_opts.no_color,
        help: pre_opts.help || post_opts.help,
    };

    if global_opts.help {
        print_line(&usage());
        return Ok(());
    }

    let db_path = resolve_db_path(global_opts.db.as_deref())?;
    let today = resolve_today(global_opts.today.as_deref())?;
    let styler = Styler::new(resolve_color_enabled(global_opts.no_color));

    if cmd == "add" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("schedule", ArgSpec::Value);
        allowed.insert("period", ArgSpec::Value);
        allowed.insert("target", ArgSpec::Value);
        allowed.insert("notes", ArgSpec::Value);
        allowed.insert("format", ArgSpec::Enum(&["table", "json"]));

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage(
                "Usage: habit add <name> [--schedule ...] [--target N] [--period day|week] [--notes ...]",
            ));
        }

        let name = parsed
            .positionals
            .get(0)
            .ok_or_else(|| CliError::usage("Habit name is required"))?;
        if parsed.positionals.len() > 1 {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .or(global_opts.format.as_deref())
            .unwrap_or("table");

        let schedule = opt_str(&parsed, "schedule").unwrap_or("everyday");
        let period = opt_str(&parsed, "period").unwrap_or("day");
        let target = opt_str(&parsed, "target")
            .map(|s| s.parse::<u32>().map_err(|_| CliError::usage("Invalid target")))
            .transpose()?
            .unwrap_or(1);
        let notes = opt_str(&parsed, "notes");

        let created = update_db(&db_path, |db| {
            let id = next_habit_id(db);
            let habit = make_habit(id, name, schedule, period, target, notes, &today)?;
            db.habits.push(habit.clone());
            Ok(habit)
        })?;

        if format == "json" {
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
            print_line(&render_simple_table(&["id", "name", "schedule", "target"], &[row]));
        }

        return Ok(());
    }

    if cmd == "list" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("all", ArgSpec::Bool);
        allowed.insert("format", ArgSpec::Enum(&["table", "json"]));

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage("Usage: habit list [--all] [--format table|json]"));
        }
        if !parsed.positionals.is_empty() {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .or(global_opts.format.as_deref())
            .unwrap_or("table");

        let db = read_db(&db_path)?;
        let habits = list_habits(&db, opt_bool(&parsed, "all"));

        if format == "json" {
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
                        if h.archived { "yes".to_string() } else { "no".to_string() },
                    ]
                })
                .collect();

            print_line(&render_simple_table(
                &["id", "name", "schedule", "target", "archived"],
                &rows,
            ));
        }

        return Ok(());
    }

    if cmd == "show" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("format", ArgSpec::Enum(&["table", "json"]));

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage("Usage: habit show <habit>"));
        }

        let sel = parsed
            .positionals
            .get(0)
            .ok_or_else(|| CliError::usage("Habit selector is required"))?;
        if parsed.positionals.len() > 1 {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .or(global_opts.format.as_deref())
            .unwrap_or("table");

        let db = read_db(&db_path)?;
        let idx = select_habit_index(&db, sel, true)?;
        let habit = db.habits[idx].clone();
        let checkins = list_checkins_for_habit(&db, &habit.id);

        if format == "json" {
            #[derive(serde::Serialize)]
            struct Out {
                habit: crate::model::Habit,
                checkins: Vec<crate::model::Checkin>,
            }
            print_json(&Out { habit, checkins })?;
        } else {
            print_line(&format!("{} ({})", habit.name, habit.id));
            print_line(&format!("schedule: {}", schedule_to_string(&habit.schedule)));
            print_line(&format!("target: {}/{}", habit.target.quantity, habit.target.period));
            print_line(&format!("archived: {}", if habit.archived { "yes" } else { "no" }));
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

        return Ok(());
    }

    if cmd == "archive" || cmd == "unarchive" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("format", ArgSpec::Enum(&["table", "json"]));

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage(format!("Usage: habit {} <habit>", cmd)));
        }

        let sel = parsed
            .positionals
            .get(0)
            .ok_or_else(|| CliError::usage("Habit selector is required"))?;
        if parsed.positionals.len() > 1 {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .or(global_opts.format.as_deref())
            .unwrap_or("table");

        let updated = update_db(&db_path, |db| {
            let idx = select_habit_index(db, sel, true)?;
            let habit = &mut db.habits[idx];

            if cmd == "archive" {
                habit.archived = true;
                if habit.archived_date.is_none() {
                    habit.archived_date = Some(today.clone());
                }
            } else {
                habit.archived = false;
                habit.archived_date = None;
            }

            Ok(habit.clone())
        })?;

        if format == "json" {
            #[derive(serde::Serialize)]
            struct Out {
                habit: crate::model::Habit,
            }
            print_json(&Out { habit: updated })?;
        } else {
            let action = if cmd == "archive" { "Archived" } else { "Unarchived" };
            print_line(&format!("{}: {} ({})", action, updated.name, updated.id));
        }

        return Ok(());
    }

    if cmd == "checkin" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("date", ArgSpec::Value);
        allowed.insert("qty", ArgSpec::Value);
        allowed.insert("set", ArgSpec::Value);
        allowed.insert("delete", ArgSpec::Bool);
        allowed.insert("format", ArgSpec::Enum(&["table", "json"]));

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage(
                "Usage: habit checkin <habit> [--date YYYY-MM-DD] [--qty N] [--set N] [--delete]",
            ));
        }

        let sel = parsed
            .positionals
            .get(0)
            .ok_or_else(|| CliError::usage("Habit selector is required"))?;
        if parsed.positionals.len() > 1 {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .or(global_opts.format.as_deref())
            .unwrap_or("table");

        let date = opt_str(&parsed, "date").unwrap_or(&today);
        parse_date_string(date, "date")?;

        let has_qty = parsed.opts.contains_key("qty");
        let has_set = parsed.opts.contains_key("set");
        let has_delete = opt_bool(&parsed, "delete");

        if has_delete && (has_qty || has_set) {
            return Err(CliError::usage("Invalid flags: --delete conflicts with --qty/--set"));
        }
        if has_qty && has_set {
            return Err(CliError::usage("Invalid flags: --qty conflicts with --set"));
        }

        let qty: u32 = if has_qty {
            let v = opt_str(&parsed, "qty").unwrap();
            v.parse().map_err(|_| CliError::usage("Invalid quantity"))?
        } else {
            1
        };

        let set: u32 = if has_set {
            let v = opt_str(&parsed, "set").unwrap();
            v.parse().map_err(|_| CliError::usage("Invalid quantity"))?
        } else {
            0
        };

        if has_qty && qty < 1 {
            return Err(CliError::usage("Invalid quantity"));
        }

        #[derive(Debug, Clone)]
        struct ResultRow {
            habit_id: String,
            habit_name: String,
            date: String,
            action: String,
            delta: Option<u32>,
            quantity: u32,
        }

        let result = update_db(&db_path, |db| {
            let idx = select_habit_index(db, sel, true)?;
            let habit = db.habits[idx].clone();

            if has_delete {
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

            if has_set {
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

        if format == "json" {
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
                habit: HabitMini { id: result.habit_id, name: result.habit_name },
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

        return Ok(());
    }

    if cmd == "status" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("date", ArgSpec::Value);
        allowed.insert("week-of", ArgSpec::Value);
        allowed.insert("include-archived", ArgSpec::Bool);
        allowed.insert("format", ArgSpec::Enum(&["table", "json"]));

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage(
                "Usage: habit status [--date YYYY-MM-DD] [--week-of YYYY-MM-DD] [--include-archived]",
            ));
        }
        if !parsed.positionals.is_empty() {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .or(global_opts.format.as_deref())
            .unwrap_or("table");

        let date = opt_str(&parsed, "date").unwrap_or(&today);
        parse_date_string(date, "date")?;

        let week_of = opt_str(&parsed, "week-of");
        if let Some(wo) = week_of {
            parse_date_string(wo, "week-of")?;
        }

        let db = read_db(&db_path)?;
        let data = build_status(&db, date, week_of, opt_bool(&parsed, "include-archived"))?;

        if format == "json" {
            print_json(&data)?;
        } else {
            print_line(&format!("Today ({})", data.today.date));
            if data.today.habits.is_empty() {
                print_line(&styler.gray("(no scheduled habits)"));
            } else {
                for h in data.today.habits.iter() {
                    let mark = if h.done { styler.green("[x]") } else { "[ ]".to_string() };
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
                            r.name,
                            r.quantity,
                            r.target
                        ));
                    }
                }
            }
        }

        return Ok(());
    }

    if cmd == "stats" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("from", ArgSpec::Value);
        allowed.insert("to", ArgSpec::Value);
        allowed.insert("format", ArgSpec::Enum(&["table", "json"]));

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage(
                "Usage: habit stats [<habit>] [--from YYYY-MM-DD] [--to YYYY-MM-DD]",
            ));
        }

        let selector = parsed.positionals.get(0).map(|s| s.as_str());
        if parsed.positionals.len() > 1 {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .or(global_opts.format.as_deref())
            .unwrap_or("table");

        let db = read_db(&db_path)?;

        let habits: Vec<crate::model::Habit> = if let Some(sel) = selector {
            let idx = select_habit_index(&db, sel, true)?;
            vec![db.habits[idx].clone()]
        } else {
            db.habits.iter().filter(|h| !h.archived).cloned().collect()
        };

        let from_opt = opt_str(&parsed, "from").map(|s| s.to_string());
        let to_opt = opt_str(&parsed, "to").map(|s| s.to_string());

        if let Some(ref t) = to_opt {
            parse_date_string(t, "to")?;
        }
        if let Some(ref f) = from_opt {
            parse_date_string(f, "from")?;
        }

        let to_eff = to_opt.unwrap_or_else(|| today.clone());

        let (from_eff, to_eff2) = if from_opt.is_none() {
            let all_week = habits.iter().all(|h| h.target.period == "week");
            if all_week {
                let end_week = iso_week_start(&to_eff)?;
                let from2 = add_days(&end_week, -7 * (12 - 1))?;
                let to2 = add_days(&end_week, 6)?;
                (from2, to2)
            } else {
                let from2 = add_days(&to_eff, -29)?;
                (from2, to_eff.clone())
            }
        } else {
            (from_opt.unwrap(), to_eff.clone())
        };

        let rows = build_stats(&db, &habits, &from_eff, &to_eff2)?;

        if format == "json" {
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
                        rate,
                        r.success_rate.successes,
                        r.success_rate.eligible
                    ),
                ]);
            }

            print_line(&render_simple_table(
                &["id", "name", "period", "current", "longest", "success"],
                &table_rows,
            ));
        }

        return Ok(());
    }

    if cmd == "export" {
        let mut allowed: BTreeMap<&'static str, ArgSpec> = BTreeMap::new();
        allowed.insert("format", ArgSpec::Enum(&["json", "csv"]));
        allowed.insert("out", ArgSpec::Value);
        allowed.insert("from", ArgSpec::Value);
        allowed.insert("to", ArgSpec::Value);
        allowed.insert("include-archived", ArgSpec::Bool);

        let parsed = parse_command_opts(&cmd_args, &allowed)?;
        if opt_bool(&parsed, "help") {
            return Err(CliError::usage(
                "Usage: habit export --format json|csv [--out <path>] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--include-archived]",
            ));
        }
        if !parsed.positionals.is_empty() {
            return Err(CliError::usage("Too many arguments"));
        }

        let format = opt_str(&parsed, "format")
            .ok_or_else(|| CliError::usage("Missing required --format"))?;

        let from = opt_str(&parsed, "from");
        let to = opt_str(&parsed, "to");

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

        let out = opt_str(&parsed, "out");

        let db = read_db(&db_path)?;
        let habits = list_habits(&db, opt_bool(&parsed, "include-archived"));
        let habit_ids: BTreeSet<String> = habits.iter().map(|h| h.id.clone()).collect();
        let checkins = list_checkins_in_range(&db, from, to, Some(&habit_ids));

        if format == "json" {
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
            let data =
                stable_to_string_pretty(&payload).map_err(|_| CliError::io("DB IO error"))? + "\n";

            if let Some(p) = out {
                fs::write(p, data.as_bytes()).map_err(|_| CliError::io("DB IO error"))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o600));
                }
            } else {
                print!("{}", data);
            }
            return Ok(());
        }

        // CSV
        let out_dir = out.ok_or_else(|| CliError::usage("CSV export requires --out <dir>"))?;
        export_csv_to_dir(out_dir, &habits, &checkins)?;
        return Ok(());
    }

    Err(CliError::usage(format!("Unknown command: {}", cmd)))
}
