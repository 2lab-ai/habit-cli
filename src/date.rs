use crate::error::CliError;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Date {
    y: i32,
    m: u32,
    d: u32,
}

fn is_valid_date(y: i32, m: u32, d: u32) -> bool {
    if !(1..=12).contains(&m) {
        return false;
    }
    if d < 1 {
        return false;
    }

    let dim = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            if leap { 29 } else { 28 }
        }
        _ => return false,
    };

    d <= dim
}

// Howard Hinnant's algorithm: days since 1970-01-01 (Unix epoch)
fn days_from_civil(mut y: i32, m: u32, d: u32) -> i32 {
    let m = m as i32;
    let d = d as i32;
    y -= if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i32) -> Date {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
    y += if m <= 2 { 1 } else { 0 };

    Date {
        y,
        m: m as u32,
        d: d as u32,
    }
}

fn fmt_date(dt: Date) -> String {
    format!("{:04}-{:02}-{:02}", dt.y, dt.m, dt.d)
}

fn parse_date(s: &str, label: &str) -> Result<Date, CliError> {
    let ss = s.trim();
    if ss.len() != 10 {
        return Err(CliError::usage(format!("Invalid {}: {}", label, s)));
    }
    let bytes = ss.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(CliError::usage(format!("Invalid {}: {}", label, s)));
    }

    let y: i32 = ss[0..4]
        .parse()
        .map_err(|_| CliError::usage(format!("Invalid {}: {}", label, s)))?;
    let m: u32 = ss[5..7]
        .parse()
        .map_err(|_| CliError::usage(format!("Invalid {}: {}", label, s)))?;
    let d: u32 = ss[8..10]
        .parse()
        .map_err(|_| CliError::usage(format!("Invalid {}: {}", label, s)))?;

    if !is_valid_date(y, m, d) {
        return Err(CliError::usage(format!("Invalid {}: {}", label, s)));
    }

    Ok(Date { y, m, d })
}

pub fn parse_date_string(s: &str, label: &str) -> Result<(), CliError> {
    let _ = parse_date(s, label)?;
    Ok(())
}

pub fn add_days(date: &str, delta_days: i32) -> Result<String, CliError> {
    let dt = parse_date(date, "date")?;
    let days = days_from_civil(dt.y, dt.m, dt.d);
    Ok(fmt_date(civil_from_days(days + delta_days)))
}

/// ISO weekday number: Mon=1..Sun=7
pub fn iso_weekday(date: &str) -> Result<u8, CliError> {
    let dt = parse_date(date, "date")?;
    let days = days_from_civil(dt.y, dt.m, dt.d);
    Ok(((days + 3).rem_euclid(7) + 1) as u8)
}

pub fn date_range_inclusive(from: &str, to: &str) -> Result<Vec<String>, CliError> {
    parse_date_string(from, "from")?;
    parse_date_string(to, "to")?;
    if from > to {
        return Err(CliError::usage("Invalid range: from > to"));
    }

    let from_dt = parse_date(from, "from")?;
    let to_dt = parse_date(to, "to")?;

    let mut cur = days_from_civil(from_dt.y, from_dt.m, from_dt.d);
    let end = days_from_civil(to_dt.y, to_dt.m, to_dt.d);

    let mut out = Vec::new();
    while cur <= end {
        out.push(fmt_date(civil_from_days(cur)));
        cur += 1;
    }
    Ok(out)
}

pub fn iso_week_start(date: &str) -> Result<String, CliError> {
    let wd = iso_weekday(date)? as i32;
    add_days(date, -(wd - 1))
}

pub fn iso_week_end(date: &str) -> Result<String, CliError> {
    let start = iso_week_start(date)?;
    add_days(&start, 6)
}

pub fn iso_week_id(week_start_date: &str) -> Result<String, CliError> {
    // week_year is the year of Thursday in that ISO week.
    let wd = iso_weekday(week_start_date)? as i32;
    let thursday = add_days(week_start_date, 4 - wd)?;
    let th_dt = parse_date(&thursday, "date")?;
    let week_year = th_dt.y;

    let jan4 = format!("{:04}-01-04", week_year);
    let week1_monday = iso_week_start(&jan4)?;

    let ws = iso_week_start(week_start_date)?;
    let ws_dt = parse_date(&ws, "date")?;
    let w1_dt = parse_date(&week1_monday, "date")?;

    let ws_days = days_from_civil(ws_dt.y, ws_dt.m, ws_dt.d);
    let w1_days = days_from_civil(w1_dt.y, w1_dt.m, w1_dt.d);

    let week = 1 + ((ws_days - w1_days) / 7);
    Ok(format!("{:04}-W{:02}", week_year, week as i32))
}

pub fn system_today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86_400) as i32;
    fmt_date(civil_from_days(days))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_parse_validation() {
        assert!(parse_date_string("2026-01-31", "today").is_ok());
        assert!(parse_date_string("2026-02-29", "today").is_err());
        assert!(parse_date_string("2024-02-29", "today").is_ok());
        assert!(parse_date_string("2026-13-01", "today").is_err());
    }

    #[test]
    fn iso_week_math_matches_expectations() {
        assert_eq!(iso_weekday("2026-01-31").unwrap(), 6); // Saturday
        assert_eq!(iso_week_start("2026-01-31").unwrap(), "2026-01-26");
        assert_eq!(iso_week_end("2026-01-31").unwrap(), "2026-02-01");
        assert_eq!(iso_week_id("2026-01-26").unwrap(), "2026-W05");
    }
}
