//! Background jobs module
//!
//! Contains scheduled tasks for maintenance and cleanup operations.
//! These are designed to be called by an external scheduler (cron, tokio-cron).

mod cleanup;

// Re-export for external scheduler use
#[allow(unused_imports)]
pub use cleanup::{CleanupError, CleanupResult, CleanupService};
