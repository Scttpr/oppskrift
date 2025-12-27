//! ActivityPub federation support
//!
//! Implements ActivityPub protocol for federated recipe sharing.
//! See: https://www.w3.org/TR/activitypub/

pub mod actor;
pub mod objects;

pub use actor::*;
pub use objects::*;

/// ActivityPub context URLs
pub const ACTIVITYSTREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
pub const SECURITY_CONTEXT: &str = "https://w3id.org/security/v1";
