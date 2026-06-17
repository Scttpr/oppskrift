use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Maximum length of a comment body (characters)
pub const MAX_COMMENT_LEN: usize = 2000;

/// A comment on a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A comment with its author's display name resolved (for rendering / listing)
#[derive(Debug, Clone, Serialize)]
pub struct CommentWithAuthor {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// Request body for posting a comment
#[derive(Debug, Deserialize)]
pub struct CreateComment {
    pub body: String,
}
