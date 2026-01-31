use crate::error::CliError;

const DAY_NAME_TO_ISO: [(&str, u8); 7] = [
    ("mon", 1),
    ("tue", 2),
    ("wed", 3),
    ("thu", 4),
    ("fri", 5),
    ("sat", 6),
    ("sun", 7),
];

fn iso_to_day_name(d: u8) -> Option<&'static str> {
    match d {
        1 => Some("mon"),
        2 => Some("tue"),
        3 => Some("wed"),
        4 => Some("thu"),
        5 => Some("fri"),
        6 => Some("sat"),
        7 => Some("sun"),
        _ => None,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    #[serde(rename = "type")]
    pub kind: String,
    pub days: Vec<u8>,
}

pub fn parse_schedule_pattern(pattern_raw: &str) -> Result<Schedule, CliError> {
    let pattern = pattern_raw.trim().to_lowercase();
    if pattern.is_empty() {
        return Err(CliError::usage("Invalid schedule pattern"));
    }

    let mut days: Vec<u8> = if pattern == "everyday" {
        vec![1, 2, 3, 4, 5, 6, 7]
    } else if pattern == "weekdays" {
        vec![1, 2, 3, 4, 5]
    } else if pattern == "weekends" {
        vec![6, 7]
    } else {
        let parts: Vec<&str> = pattern
            .split(',')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();
        if parts.is_empty() {
            return Err(CliError::usage(format!("Invalid schedule pattern: {}", pattern_raw)));
        }
        let mut out: Vec<u8> = Vec::new();
        for p in parts {
            let iso = DAY_NAME_TO_ISO
                .iter()
                .find(|(name, _)| *name == p)
                .map(|(_, d)| *d)
                .ok_or_else(|| CliError::usage(format!("Invalid schedule pattern: {}", pattern_raw)))?;
            if !out.contains(&iso) {
                out.push(iso);
            }
        }
        out.sort();
        out
    };

    days.sort();

    Ok(Schedule {
        kind: "days_of_week".to_string(),
        days,
    })
}

pub fn schedule_to_string(schedule: &Schedule) -> String {
    let mut days = schedule.days.clone();
    days.sort();

    let is_everyday = days.len() == 7 && days.iter().enumerate().all(|(i, d)| *d as usize == i + 1);
    let is_weekdays = days.len() == 5 && days.iter().enumerate().all(|(i, d)| *d as usize == i + 1);
    let is_weekends = days.len() == 2 && days[0] == 6 && days[1] == 7;

    if is_everyday {
        return "everyday".to_string();
    }
    if is_weekdays {
        return "weekdays".to_string();
    }
    if is_weekends {
        return "weekends".to_string();
    }

    days.iter()
        .filter_map(|d| iso_to_day_name(*d))
        .collect::<Vec<&str>>()
        .join(",")
}

pub fn validate_schedule(schedule: &Schedule) -> Result<(), CliError> {
    if schedule.kind != "days_of_week" {
        return Err(CliError::usage("Invalid schedule"));
    }
    if schedule.days.is_empty() {
        return Err(CliError::usage("Invalid schedule"));
    }
    for d in schedule.days.iter() {
        if *d < 1 || *d > 7 {
            return Err(CliError::usage("Invalid schedule"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_patterns_roundtrip() {
        assert_eq!(schedule_to_string(&parse_schedule_pattern("everyday").unwrap()), "everyday");
        assert_eq!(schedule_to_string(&parse_schedule_pattern("weekdays").unwrap()), "weekdays");
        assert_eq!(schedule_to_string(&parse_schedule_pattern("weekends").unwrap()), "weekends");
        assert_eq!(schedule_to_string(&parse_schedule_pattern("mon,wed,fri").unwrap()), "mon,wed,fri");
    }
}
