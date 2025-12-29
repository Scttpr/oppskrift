//! Helper utilities
//!
//! Provides common helper functions used across the application.

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
