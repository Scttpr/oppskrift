//! Helper utilities
//!
//! Provides common helper functions used across the application.

use chrono::{DateTime, Datelike, Timelike, Utc};

const FRENCH_MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

fn french_month(dt: &DateTime<Utc>) -> &'static str {
    FRENCH_MONTHS[(dt.month0()) as usize]
}

/// Format a datetime in French, e.g. "25 décembre 2025 à 14h30".
pub fn format_fr_datetime(dt: &DateTime<Utc>) -> String {
    format!(
        "{} {} {} à {:02}h{:02}",
        dt.day(),
        french_month(dt),
        dt.year(),
        dt.hour(),
        dt.minute()
    )
}

/// Format a month and year in French, e.g. "décembre 2025".
pub fn format_fr_month_year(dt: &DateTime<Utc>) -> String {
    format!("{} {}", french_month(dt), dt.year())
}

/// Format a date in French (no time), e.g. "25 décembre 2025".
pub fn format_fr_date(dt: &DateTime<Utc>) -> String {
    format!("{} {} {}", dt.day(), french_month(dt), dt.year())
}

/// Mask an email address for display
///
/// Shows the first character, asterisks, and the domain.
/// Example: "john.doe@example.com" -> "j*******@example.com"
///
/// Returns the original email if it's malformed.
pub fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return email.to_string();
    };

    if local.is_empty() {
        return email.to_string();
    }

    let first_char: String = local.chars().take(1).collect();
    let mask_len = local.len().saturating_sub(1);
    let masked = "*".repeat(mask_len.min(7)); // Cap at 7 asterisks

    format!("{}{}@{}", first_char, masked, domain)
}

/// Mask an email domain for audit logs
///
/// Example: "john.doe@example.com" -> "example.com"
pub fn extract_email_domain(email: &str) -> Option<&str> {
    email.split_once('@').map(|(_, domain)| domain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_format_fr_datetime() {
        let dt = Utc.with_ymd_and_hms(2025, 12, 25, 14, 30, 0).unwrap();
        assert_eq!(format_fr_datetime(&dt), "25 décembre 2025 à 14h30");
    }

    #[test]
    fn test_format_fr_datetime_pads_time_not_day() {
        let dt = Utc.with_ymd_and_hms(2025, 1, 5, 9, 5, 0).unwrap();
        assert_eq!(format_fr_datetime(&dt), "5 janvier 2025 à 09h05");
    }

    #[test]
    fn test_format_fr_month_year() {
        let dt = Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap();
        assert_eq!(format_fr_month_year(&dt), "décembre 2025");
    }

    #[test]
    fn test_format_fr_date() {
        let dt = Utc.with_ymd_and_hms(2025, 12, 25, 14, 30, 0).unwrap();
        assert_eq!(format_fr_date(&dt), "25 décembre 2025");
    }

    #[test]
    fn test_mask_email_normal() {
        assert_eq!(mask_email("john.doe@example.com"), "j*******@example.com");
    }

    #[test]
    fn test_mask_email_short() {
        assert_eq!(mask_email("j@example.com"), "j@example.com");
    }

    #[test]
    fn test_mask_email_two_chars() {
        assert_eq!(mask_email("jo@example.com"), "j*@example.com");
    }

    #[test]
    fn test_mask_email_long_local() {
        assert_eq!(
            mask_email("verylongusername@example.com"),
            "v*******@example.com"
        );
    }

    #[test]
    fn test_mask_email_malformed() {
        assert_eq!(mask_email("not-an-email"), "not-an-email");
    }

    #[test]
    fn test_mask_email_empty_local() {
        assert_eq!(mask_email("@example.com"), "@example.com");
    }

    #[test]
    fn test_extract_email_domain() {
        assert_eq!(
            extract_email_domain("john@example.com"),
            Some("example.com")
        );
    }

    #[test]
    fn test_extract_email_domain_malformed() {
        assert_eq!(extract_email_domain("not-an-email"), None);
    }
}
