// Lib module - Shared utilities, Schema.org serialization

pub mod db;
pub mod error;
pub mod pagination;
pub mod schema_org;
pub mod seeds;

pub use error::{AppError, AppResult, ErrorResponse};
pub use pagination::{PaginatedResponse, PaginationMeta, PaginationParams};
pub use schema_org::SchemaOrgRecipe;
