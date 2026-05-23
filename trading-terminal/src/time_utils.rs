use chrono::{TimeZone, Utc};

/// Convert a unix timestamp (seconds) into a human-readable "age" string like `5m ago`.
pub fn format_unix_time(unix_time: u64) -> String {
    use std::time::{UNIX_EPOCH, Duration};
    let duration = Duration::from_secs(unix_time);
    let datetime = UNIX_EPOCH + duration;
    let now = std::time::SystemTime::now();

    if let Ok(elapsed) = now.duration_since(datetime) {
        let secs = elapsed.as_secs();
        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    } else {
        "Just now".to_string()
    }
}

/// Convert unix timestamp to a full date/time string for display in the "Date" column.
pub fn format_full_time(unix_time: u64) -> String {
    let dt = Utc.timestamp_opt(unix_time as i64, 0)
        .single()
        .unwrap_or_else(|| Utc::now());
    dt.format("%Y-%m-%d %H:%M").to_string()
}
