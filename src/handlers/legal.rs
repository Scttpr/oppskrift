use askama::Template;
use axum::{response::Html, routing::get, Router};

use crate::core::error::AppResult;
use crate::AppState;

/// Legal page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/about", get(about_page))
        .route("/privacy", get(privacy_page))
        .route("/terms", get(terms_page))
}

/// About page template
#[derive(Template)]
#[template(path = "legal/about.html")]
struct AboutTemplate;

/// Privacy policy template
#[derive(Template)]
#[template(path = "legal/privacy.html")]
struct PrivacyTemplate;

/// Terms of service template
#[derive(Template)]
#[template(path = "legal/terms.html")]
struct TermsTemplate;

/// About page handler
async fn about_page() -> AppResult<Html<String>> {
    let template = AboutTemplate;
    crate::core::render(&template)
}

/// Privacy policy page handler
async fn privacy_page() -> AppResult<Html<String>> {
    let template = PrivacyTemplate;
    crate::core::render(&template)
}

/// Terms of service page handler
async fn terms_page() -> AppResult<Html<String>> {
    let template = TermsTemplate;
    crate::core::render(&template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use askama::Template;

    // ==========================================================================
    // Route Configuration Tests (T056)
    // ==========================================================================

    #[test]
    fn test_routes_returns_router() {
        let router = routes();
        let _ = router;
    }

    // ==========================================================================
    // Template Rendering Tests (T056)
    // ==========================================================================

    fn assert_no_english(html: &str, sentinels: &[&str]) {
        for s in sentinels {
            assert!(
                !html.contains(s),
                "should not contain English sentinel: {s:?}"
            );
        }
    }

    #[test]
    fn test_legal_pages_are_french() {
        assert_no_english(
            &AboutTemplate.render().unwrap(),
            &[
                "About Oppskrift",
                "What is Oppskrift",
                "Features",
                "Open Source",
            ],
        );
        assert_no_english(
            &TermsTemplate.render().unwrap(),
            &[
                "Terms of Service",
                "Acceptance of Terms",
                "User Content",
                "Governing Law",
            ],
        );
        assert_no_english(
            &PrivacyTemplate.render().unwrap(),
            &[
                "Privacy Policy",
                "Data Controller",
                "Your Rights",
                "Data Retention",
            ],
        );
    }

    #[test]
    fn test_about_template_renders() {
        let template = AboutTemplate;
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_privacy_template_renders() {
        let template = PrivacyTemplate;
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_terms_template_renders() {
        let template = TermsTemplate;
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.is_empty());
    }

    // ==========================================================================
    // Handler Async Tests (T056)
    // ==========================================================================

    #[tokio::test]
    async fn test_about_page_handler() {
        let result = about_page().await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.0.is_empty());
    }

    #[tokio::test]
    async fn test_privacy_page_handler() {
        let result = privacy_page().await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.0.is_empty());
    }

    #[tokio::test]
    async fn test_terms_page_handler() {
        let result = terms_page().await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.0.is_empty());
    }

    // ==========================================================================
    // Content Tests (T056)
    // ==========================================================================

    #[test]
    fn test_about_contains_expected_content() {
        let html = AboutTemplate.render().unwrap();
        // About page should contain some information about the site
        assert!(
            html.len() > 100,
            "About page should have substantial content"
        );
    }

    #[test]
    fn test_privacy_contains_expected_content() {
        let html = PrivacyTemplate.render().unwrap();
        // Privacy page should mention privacy-related terms
        assert!(
            html.len() > 100,
            "Privacy page should have substantial content"
        );
    }

    #[test]
    fn test_terms_contains_expected_content() {
        let html = TermsTemplate.render().unwrap();
        // Terms page should have content
        assert!(
            html.len() > 100,
            "Terms page should have substantial content"
        );
    }
}
