use chrono::{DateTime, Duration, Utc};

pub fn is_interval_due(
    last_check_at: Option<&str>,
    interval_hours: u64,
    now: DateTime<Utc>,
) -> bool {
    let Some(last_check_at) = last_check_at else {
        return true;
    };
    let Ok(last) = DateTime::parse_from_rfc3339(last_check_at) else {
        return true;
    };
    now.signed_duration_since(last.with_timezone(&Utc)) >= Duration::hours(interval_hours as i64)
}

pub fn auto_download_allowed(
    auto_download_enabled: bool,
    wifi_only: bool,
    wifi_known_and_connected: bool,
) -> bool {
    if !auto_download_enabled {
        return false;
    }
    if !wifi_only {
        return true;
    }
    wifi_known_and_connected
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{auto_download_allowed, is_interval_due};

    #[test]
    fn interval_due_is_false_when_last_check_is_recent() {
        let now = Utc::now();
        let last = (now - Duration::hours(1)).to_rfc3339();
        assert!(!is_interval_due(Some(&last), 24, now));
    }

    #[test]
    fn interval_due_is_true_when_missing_or_expired() {
        let now = Utc::now();
        let last = (now - Duration::hours(30)).to_rfc3339();
        assert!(is_interval_due(None, 24, now));
        assert!(is_interval_due(Some(&last), 24, now));
    }

    #[test]
    fn wifi_only_policy_denies_when_network_unknown() {
        assert!(!auto_download_allowed(true, true, false));
        assert!(auto_download_allowed(true, false, false));
        assert!(!auto_download_allowed(false, false, true));
    }
}
