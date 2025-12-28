//! Request ID middleware and request context
//!
//! Generates a unique UUID for each HTTP request and stores it in request extensions.
//! This allows correlating all logs and audit events from a single request.

use axum::{extract::Request, middleware::Next, response::Response};
use std::net::IpAddr;
use uuid::Uuid;

/// Request ID stored in request extensions
#[derive(Debug, Clone, Copy)]
pub struct RequestId(pub Uuid);

/// Context for a request, bundling tracing and identity information
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// Unique ID for this HTTP request (correlates all events)
    pub request_id: Option<Uuid>,
    /// Client IP address
    pub ip: Option<IpAddr>,
    /// Session ID (if authenticated)
    pub session_id: Option<Uuid>,
}

impl RequestContext {
    /// Create a new empty request context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the request ID
    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Set optional request ID
    pub fn maybe_request_id(mut self, request_id: Option<Uuid>) -> Self {
        self.request_id = request_id;
        self
    }

    /// Set the IP address
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    /// Set optional IP address
    pub fn maybe_ip(mut self, ip: Option<IpAddr>) -> Self {
        self.ip = ip;
        self
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set optional session ID
    pub fn maybe_session_id(mut self, session_id: Option<Uuid>) -> Self {
        self.session_id = session_id;
        self
    }
}

impl RequestId {
    /// Generate a new request ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Middleware that generates a unique request ID for each request
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = RequestId::new();

    // Add to request extensions for downstream handlers
    request.extensions_mut().insert(request_id);

    // Create a tracing span with the request ID
    let span = tracing::info_span!(
        "request",
        request_id = %request_id.0
    );

    // Execute request within the span
    let _guard = span.enter();

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::new();
        let display = format!("{}", id);
        assert_eq!(display.len(), 36); // UUID format
    }
}
