//! Security middleware
//!
//! Provides security headers for all responses, including Content-Security-Policy.

use axum::{
    body::Body,
    http::{header, Request, Response},
    middleware::Next,
};

/// Content-Security-Policy header value
///
/// This policy:
/// - Restricts scripts to same-origin and inline (for HTMX)
/// - Restricts styles to same-origin and inline (for Tailwind)
/// - Restricts images to same-origin, data URIs, and HTTPS
/// - Restricts fonts to same-origin
/// - Restricts form actions to same-origin
/// - Restricts frame ancestors to none (prevents clickjacking)
/// - Upgrades insecure requests to HTTPS
const CSP_HEADER: &str = concat!(
    "default-src 'self'; ",
    "script-src 'self' 'unsafe-inline'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data: https:; ",
    "font-src 'self'; ",
    "form-action 'self'; ",
    "frame-ancestors 'none'; ",
    "base-uri 'self'; ",
    "upgrade-insecure-requests"
);

/// Add security headers to all responses
pub async fn security_headers(request: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Content-Security-Policy
    headers.insert(header::CONTENT_SECURITY_POLICY, CSP_HEADER.parse().unwrap());

    // Prevent MIME type sniffing
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());

    // Enable XSS filter (legacy, but still useful)
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        "1; mode=block".parse().unwrap(),
    );

    // Prevent clickjacking (backup for frame-ancestors)
    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());

    // Control referrer information
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // Permissions policy (restrict access to sensitive APIs)
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        "geolocation=(), microphone=(), camera=()".parse().unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csp_header_not_empty() {
        assert!(CSP_HEADER.contains("default-src"));
        assert!(CSP_HEADER.contains("script-src"));
        assert!(CSP_HEADER.contains("frame-ancestors 'none'"));
    }
}
