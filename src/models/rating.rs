use serde::{Deserialize, Serialize};

/// Aggregate rating information for a recipe, plus the viewer's own rating.
#[derive(Debug, Clone, Serialize)]
pub struct RatingSummary {
    /// Average rating (1.0–5.0), or `None` if there are no ratings yet.
    pub average: Option<f64>,
    /// Total number of ratings.
    pub count: i64,
    /// The current viewer's rating, if they have rated this recipe.
    pub user_rating: Option<i16>,
}

/// Request body for setting a rating.
#[derive(Debug, Deserialize)]
pub struct SetRatingRequest {
    pub value: i16,
}
