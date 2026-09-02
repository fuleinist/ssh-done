/// Format a duration in seconds as compact human-readable text: `42s`, `3m 5s`, `1h 2m`.
pub fn format_duration(total_secs: u64) -> String {
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    if hours > 0 {
        format!("{hours}h {mins}m {secs}s")
    } else if mins > 0 {
        format!("{mins}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::format_duration;

    #[test]
    fn seconds_only() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(42), "42s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn minutes_and_seconds() {
        assert_eq!(format_duration(60), "1m 0s");
        assert_eq!(format_duration(185), "3m 5s");
    }

    #[test]
    fn hours() {
        assert_eq!(format_duration(3600), "1h 0m 0s");
        assert_eq!(format_duration(3725), "1h 2m 5s");
    }
}
