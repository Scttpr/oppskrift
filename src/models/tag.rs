use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A recipe tag / category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

/// A tag together with the number of public recipes carrying it
#[derive(Debug, Clone, Serialize)]
pub struct TagWithCount {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub recipe_count: i64,
}

/// Maximum number of tags allowed on a single recipe
pub const MAX_TAGS_PER_RECIPE: usize = 15;

/// Maximum length of a tag name (characters)
pub const MAX_TAG_NAME_LEN: usize = 50;

/// Normalize a free-form tag name into a URL-safe slug.
///
/// Lowercases, trims, and collapses any run of non-alphanumeric characters
/// into a single hyphen. Returns `None` if nothing usable remains.
pub fn slugify(name: &str) -> Option<String> {
    let mut slug = String::with_capacity(name.len());
    let mut prev_dash = false;
    for ch in name.trim().chars() {
        if ch.is_alphanumeric() {
            slug.extend(ch.to_lowercase());
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        None
    } else {
        slug.truncate(MAX_TAG_NAME_LEN);
        let trimmed = slug.trim_end_matches('-').to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Dessert").as_deref(), Some("dessert"));
        assert_eq!(slugify("Gluten Free").as_deref(), Some("gluten-free"));
        assert_eq!(slugify("  Quick & Easy  ").as_deref(), Some("quick-easy"));
        assert_eq!(
            slugify("30-Minute Meals").as_deref(),
            Some("30-minute-meals")
        );
    }

    #[test]
    fn slugify_collapses_separators() {
        assert_eq!(slugify("a---b").as_deref(), Some("a-b"));
        assert_eq!(slugify("!!!hello!!!").as_deref(), Some("hello"));
    }

    #[test]
    fn slugify_empty() {
        assert_eq!(slugify(""), None);
        assert_eq!(slugify("   "), None);
        assert_eq!(slugify("!!!"), None);
    }
}
