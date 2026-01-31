use crate::error::CliError;

pub fn validate_rfc3339(ts: &str, label: &str) -> Result<(), CliError> {
    let t = ts.trim();
    if t.is_empty() {
        return Err(CliError::usage(format!("Invalid {}: (empty)", label)));
    }
    chrono::DateTime::parse_from_rfc3339(t)
        .map(|_| ())
        .map_err(|_| CliError::usage(format!("Invalid {}: {}", label, ts)))
}
