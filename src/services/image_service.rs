use image::{imageops::FilterType, DynamicImage, ImageFormat};
use sqlx::PgPool;
use std::io::Cursor;
use uuid::Uuid;

use crate::lib::error::{AppError, AppResult};
use crate::lib::storage::StorageClient;
use crate::models::{CreateRecipeImage, RecipeImage};

/// Maximum number of images per recipe
pub const MAX_IMAGES_PER_RECIPE: usize = 10;

/// Maximum image dimension (width or height)
pub const MAX_IMAGE_DIMENSION: u32 = 2048;

/// Thumbnail dimension
pub const THUMBNAIL_SIZE: u32 = 300;

/// Allowed image MIME types
pub const ALLOWED_MIME_TYPES: [&str; 4] = [
    "image/jpeg",
    "image/png",
    "image/webp",
    "image/gif",
];

/// Service for image upload and management
pub struct ImageService;

impl ImageService {
    /// Validate that a MIME type is allowed
    pub fn validate_mime_type(mime_type: &str) -> AppResult<()> {
        if !ALLOWED_MIME_TYPES.contains(&mime_type) {
            return Err(AppError::Validation(format!(
                "Image type '{}' is not allowed. Allowed types: {}",
                mime_type,
                ALLOWED_MIME_TYPES.join(", ")
            )));
        }
        Ok(())
    }

    /// Validate image count for a recipe
    pub async fn validate_image_count(pool: &PgPool, recipe_id: Uuid) -> AppResult<()> {
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM recipe_images WHERE recipe_id = $1",
            recipe_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        if count >= MAX_IMAGES_PER_RECIPE as i64 {
            return Err(AppError::Validation(format!(
                "Recipe cannot have more than {} images",
                MAX_IMAGES_PER_RECIPE
            )));
        }

        Ok(())
    }

    /// Get the next available position for a recipe image
    async fn get_next_position(pool: &PgPool, recipe_id: Uuid) -> AppResult<i32> {
        let max_position: Option<i32> = sqlx::query_scalar!(
            "SELECT MAX(position) FROM recipe_images WHERE recipe_id = $1",
            recipe_id
        )
        .fetch_one(pool)
        .await?;

        Ok(max_position.unwrap_or(0) + 1)
    }

    /// Process and resize image if needed
    pub fn process_image(data: &[u8]) -> AppResult<(Vec<u8>, String)> {
        let img = image::load_from_memory(data)
            .map_err(|e| AppError::Validation(format!("Invalid image: {}", e)))?;

        let (width, height) = (img.width(), img.height());

        // Resize if too large
        let processed = if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
            img.resize(
                MAX_IMAGE_DIMENSION,
                MAX_IMAGE_DIMENSION,
                FilterType::Lanczos3,
            )
        } else {
            img
        };

        // Encode as WebP for optimal size
        let mut output = Vec::new();
        processed
            .write_to(&mut Cursor::new(&mut output), ImageFormat::WebP)
            .map_err(|e| AppError::Internal(format!("Failed to encode image: {}", e)))?;

        Ok((output, "image/webp".to_string()))
    }

    /// Create a thumbnail from an image
    pub fn create_thumbnail(data: &[u8]) -> AppResult<Vec<u8>> {
        let img = image::load_from_memory(data)
            .map_err(|e| AppError::Validation(format!("Invalid image: {}", e)))?;

        let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);

        let mut output = Vec::new();
        thumbnail
            .write_to(&mut Cursor::new(&mut output), ImageFormat::WebP)
            .map_err(|e| AppError::Internal(format!("Failed to create thumbnail: {}", e)))?;

        Ok(output)
    }

    /// Upload an image for a recipe
    pub async fn upload_image(
        pool: &PgPool,
        storage: &StorageClient,
        recipe_id: Uuid,
        data: Vec<u8>,
        alt_text: Option<String>,
        is_primary: bool,
    ) -> AppResult<RecipeImage> {
        // Validate image count
        Self::validate_image_count(pool, recipe_id).await?;

        // Process the image
        let (processed_data, content_type) = Self::process_image(&data)?;

        // Generate storage key
        let key = StorageClient::generate_image_key(recipe_id, "webp");

        // Upload to storage
        let url = storage.upload(&key, processed_data, &content_type).await?;

        // Get next position
        let position = Self::get_next_position(pool, recipe_id).await?;

        // If this is the primary image, unset any existing primary
        if is_primary {
            sqlx::query!(
                "UPDATE recipe_images SET is_primary = false WHERE recipe_id = $1 AND is_primary = true",
                recipe_id
            )
            .execute(pool)
            .await?;
        }

        // Insert into database
        let image = sqlx::query_as!(
            RecipeImage,
            r#"
            INSERT INTO recipe_images (recipe_id, url, alt_text, position, is_primary)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, recipe_id, url, alt_text, position, is_primary
            "#,
            recipe_id,
            url,
            alt_text,
            position,
            is_primary
        )
        .fetch_one(pool)
        .await?;

        Ok(image)
    }

    /// Get all images for a recipe
    pub async fn get_images(pool: &PgPool, recipe_id: Uuid) -> AppResult<Vec<RecipeImage>> {
        let images = sqlx::query_as!(
            RecipeImage,
            r#"
            SELECT id, recipe_id, url, alt_text, position, is_primary
            FROM recipe_images
            WHERE recipe_id = $1
            ORDER BY position
            "#,
            recipe_id
        )
        .fetch_all(pool)
        .await?;

        Ok(images)
    }

    /// Get the primary image for a recipe
    pub async fn get_primary_image(
        pool: &PgPool,
        recipe_id: Uuid,
    ) -> AppResult<Option<RecipeImage>> {
        let image = sqlx::query_as!(
            RecipeImage,
            r#"
            SELECT id, recipe_id, url, alt_text, position, is_primary
            FROM recipe_images
            WHERE recipe_id = $1 AND is_primary = true
            "#,
            recipe_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(image)
    }

    /// Delete an image
    pub async fn delete_image(
        pool: &PgPool,
        storage: &StorageClient,
        image_id: Uuid,
    ) -> AppResult<()> {
        // Get the image to find its URL
        let image = sqlx::query_as!(
            RecipeImage,
            "SELECT id, recipe_id, url, alt_text, position, is_primary FROM recipe_images WHERE id = $1",
            image_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Image {} not found", image_id)))?;

        // Extract key from URL (assuming URL ends with the key)
        let key = image.url.rsplit('/').take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/");

        // Delete from storage
        storage.delete(&key).await?;

        // Delete from database
        sqlx::query!("DELETE FROM recipe_images WHERE id = $1", image_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Set an image as primary
    pub async fn set_primary(pool: &PgPool, image_id: Uuid) -> AppResult<RecipeImage> {
        // Get the image to find recipe_id
        let image = sqlx::query_as!(
            RecipeImage,
            "SELECT id, recipe_id, url, alt_text, position, is_primary FROM recipe_images WHERE id = $1",
            image_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Image {} not found", image_id)))?;

        // Unset current primary
        sqlx::query!(
            "UPDATE recipe_images SET is_primary = false WHERE recipe_id = $1",
            image.recipe_id
        )
        .execute(pool)
        .await?;

        // Set new primary
        let updated = sqlx::query_as!(
            RecipeImage,
            r#"
            UPDATE recipe_images SET is_primary = true WHERE id = $1
            RETURNING id, recipe_id, url, alt_text, position, is_primary
            "#,
            image_id
        )
        .fetch_one(pool)
        .await?;

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_mime_type_ok() {
        assert!(ImageService::validate_mime_type("image/jpeg").is_ok());
        assert!(ImageService::validate_mime_type("image/png").is_ok());
        assert!(ImageService::validate_mime_type("image/webp").is_ok());
    }

    #[test]
    fn test_validate_mime_type_invalid() {
        assert!(ImageService::validate_mime_type("image/svg+xml").is_err());
        assert!(ImageService::validate_mime_type("application/pdf").is_err());
        assert!(ImageService::validate_mime_type("text/html").is_err());
    }
}
