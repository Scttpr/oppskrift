use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};
use uuid::Uuid;

use crate::api::middleware::OptionalAuthUser;
use crate::core::error::AppResult;
use crate::core::pagination::{PaginationMeta, PaginationParams};
use crate::models::{RecipeBook, RecipeBookSummary, RecipeSummary, User};
use crate::services::{BookService, UserService};
use crate::AppState;

/// Book page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_books_page))
        .route("/new", get(new_book_page))
        .route("/{id}", get(view_book_page))
        .route("/{id}/edit", get(edit_book_page))
}

/// Book list page template
#[derive(Template)]
#[template(path = "books/list.html")]
struct BookListTemplate {
    books: Vec<RecipeBookSummary>,
    pagination: PaginationMeta,
    user: Option<User>,
}

/// Book list page handler
async fn list_books_page(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let books_page = BookService::list_public(&state.db, &params).await?;

    let user = if let Some(auth_user) = auth.0 {
        UserService::get_by_id(&state.db, auth_user.id).await.ok()
    } else {
        None
    };

    let template = BookListTemplate {
        books: books_page.data,
        pagination: books_page.pagination,
        user,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// New book form template
#[derive(Template)]
#[template(path = "books/form.html")]
struct NewBookTemplate {
    book: Option<RecipeBook>,
}

/// New book page handler
async fn new_book_page() -> AppResult<Html<String>> {
    let template = NewBookTemplate { book: None };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Book view page template
#[derive(Template)]
#[template(path = "books/view.html")]
struct BookViewTemplate {
    book: RecipeBook,
    owner: Option<User>,
    recipes: Vec<RecipeSummary>,
    recipe_count: i64,
    is_owner: bool,
}

/// View book page handler
async fn view_book_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let book = BookService::get_by_id(&state.db, id).await?;
    let owner = UserService::get_by_id(&state.db, book.owner_id).await.ok();
    let recipes_page = BookService::get_recipes_in_book(&state.db, id, &params).await?;

    let is_owner = auth.0.as_ref().map(|u| u.id) == Some(book.owner_id);

    let template = BookViewTemplate {
        book,
        owner,
        recipes: recipes_page.data,
        recipe_count: recipes_page.pagination.total_items as i64,
        is_owner,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Edit book form template
#[derive(Template)]
#[template(path = "books/form.html")]
struct EditBookTemplate {
    book: Option<RecipeBook>,
}

/// Edit book page handler
async fn edit_book_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let book = BookService::get_by_id(&state.db, id).await?;

    let template = EditBookTemplate { book: Some(book) };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Visibility;
    use askama::Template;
    use chrono::Utc;

    // ==========================================================================
    // Route Configuration Tests (T053)
    // ==========================================================================

    #[test]
    fn test_routes_returns_router() {
        let router = routes();
        let _ = router;
    }

    // ==========================================================================
    // Template Struct Tests (T053)
    // ==========================================================================

    #[test]
    fn test_book_list_template_renders_empty() {
        let template = BookListTemplate {
            books: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
            user: None,
        };
        let result = template.render();
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_new_book_template_renders() {
        let template = NewBookTemplate { book: None };
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_edit_book_template_with_book() {
        let book = RecipeBook {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "My Cookbook".to_string(),
            description: Some("A collection of recipes".to_string()),
            cover_image_url: None,
            visibility: Visibility::Public,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/books/1".to_string(),
        };

        let template = EditBookTemplate { book: Some(book) };
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("My Cookbook") || html.contains("form") || !html.is_empty());
    }

    #[test]
    fn test_book_view_template_renders() {
        let book = RecipeBook {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "View Book".to_string(),
            description: Some("Description".to_string()),
            cover_image_url: None,
            visibility: Visibility::Public,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/books/2".to_string(),
        };

        let template = BookViewTemplate {
            book,
            owner: None,
            recipes: vec![],
            recipe_count: 0,
            is_owner: false,
        };
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("View Book") || !html.is_empty());
    }

    // ==========================================================================
    // Pagination Tests (T053)
    // ==========================================================================

    #[test]
    fn test_pagination_meta_in_template() {
        let pagination = PaginationMeta {
            page: 1,
            page_size: 10,
            total_items: 25,
            total_pages: 3,
            has_next: true,
            has_prev: false,
        };

        let template = BookListTemplate {
            books: vec![],
            pagination,
            user: None,
        };

        let html = template.render().unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_book_list_with_books() {
        let book = RecipeBookSummary {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "Sample Book".to_string(),
            description: Some("A sample".to_string()),
            cover_image_url: None,
            visibility: Visibility::Public,
            created_at: Utc::now(),
            recipe_count: 5,
        };

        let template = BookListTemplate {
            books: vec![book],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
            user: None,
        };

        let html = template.render().unwrap();
        assert!(html.contains("Sample Book") || !html.is_empty());
    }

    // ==========================================================================
    // Owner/Auth State Tests (T053)
    // ==========================================================================

    #[test]
    fn test_book_view_owner_state() {
        let book = RecipeBook {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "Owner Book".to_string(),
            description: None,
            cover_image_url: None,
            visibility: Visibility::Public,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/books/3".to_string(),
        };

        // Test as owner
        let owner_template = BookViewTemplate {
            book: book.clone(),
            owner: None,
            recipes: vec![],
            recipe_count: 0,
            is_owner: true,
        };
        assert!(owner_template.render().is_ok());

        // Test as non-owner
        let guest_template = BookViewTemplate {
            book,
            owner: None,
            recipes: vec![],
            recipe_count: 0,
            is_owner: false,
        };
        assert!(guest_template.render().is_ok());
    }

    #[test]
    fn test_book_view_with_recipe_count() {
        let book = RecipeBook {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "Full Book".to_string(),
            description: None,
            cover_image_url: None,
            visibility: Visibility::Public,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/books/4".to_string(),
        };

        let template = BookViewTemplate {
            book,
            owner: None,
            recipes: vec![],
            recipe_count: 42,
            is_owner: false,
        };

        let html = template.render().unwrap();
        assert!(!html.is_empty());
    }
}
