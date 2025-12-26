// Lib module - Shared utilities, Schema.org serialization

pub mod activitypub;
pub mod audit;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod pagination;
pub mod schema_org;
pub mod seeds;
pub mod storage;
pub mod units;

pub use audit::AuditEvent;
pub use config::Config;
pub use error::{AppError, AppResult, ErrorResponse};
pub use pagination::{PaginatedResponse, PaginationMeta, PaginationParams};
pub use schema_org::SchemaOrgRecipe;
pub use storage::{SharedStorage, StorageClient, StorageConfig};
pub use units::{MeasurementSystem, convert_quantity, format_quantity_for_user};
