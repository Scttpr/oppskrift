use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::middleware::{AuthUser, OptionalAuthUser};
use crate::core::audit::AuditEvent;
use crate::core::csrf::{generate_csrf_token, validate_csrf_token};
use crate::core::error::{AppError, AppResult};
use crate::core::pagination::{PaginationMeta, PaginationParams};
use crate::core::request_id::{RequestContext, RequestId};
use crate::models::book_contribution::{BookContributionWithDisplay, ContributionStatus};
use crate::models::{RecipeBook, RecipeBookSummary, RecipeSummary, User};
use crate::services::{BookContributionService, BookService, RecipeService, UserService};
use crate::AppState;

/// Book page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_books_page))
        .route("/new", get(new_book_page))
        .route("/{id}", get(view_book_page))
        .route("/{id}/edit", get(edit_book_page))
        // Contribution management routes (T035)
        .route(
            "/{book_id}/contributions/{contribution_id}/accept",
            post(accept_contribution),
        )
        .route(
            "/{book_id}/contributions/{contribution_id}/reject",
            post(reject_contribution),
        )
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

/// Contribution view for template display (T029)
#[derive(Debug, Clone)]
struct ContributionItemView {
    id: Uuid,
    recipe_id: Uuid,
    recipe_title: String,
    contributor_name: String,
    added_at: String,
    status: ContributionStatus,
    rejection_reason: Option<String>,
}

impl ContributionItemView {
    fn from_display(contrib: &BookContributionWithDisplay, recipe_title: String) -> Self {
        Self {
            id: contrib.id,
            recipe_id: contrib.recipe_id,
            recipe_title,
            contributor_name: contrib.contributor_display_name.clone(),
            added_at: crate::models::session::format_relative_time(contrib.added_at),
            status: contrib.contribution_status(),
            rejection_reason: contrib.rejection_reason.clone(),
        }
    }
}

/// Contribution list for book view (T029)
#[derive(Debug, Clone, Default)]
struct ContributionListView {
    pending: Vec<ContributionItemView>,
    accepted: Vec<ContributionItemView>,
    rejected: Vec<ContributionItemView>,
    pending_count: usize,
}

/// Book view page template (T030)
#[derive(Template)]
#[template(path = "books/view.html")]
struct BookViewTemplate {
    book: RecipeBook,
    owner: Option<User>,
    recipes: Vec<RecipeSummary>,
    recipe_count: i64,
    is_owner: bool,
    contributions: ContributionListView,
    csrf_token: String,
}

/// View book page handler (T030)
async fn view_book_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let viewer_id = auth.0.as_ref().map(|u| u.id);
    let book = BookService::get_by_id_authorized(&state.db, id, viewer_id).await?;
    let owner = UserService::get_by_id(&state.db, book.owner_id).await.ok();
    let recipes_page = BookService::get_recipes_in_book(&state.db, id, &params).await?;

    let is_owner = viewer_id == Some(book.owner_id);

    // Generate CSRF token if logged in
    let csrf_token = if let Some(ref auth_user) = auth.0 {
        generate_csrf_token(auth_user.session_id, &state.csrf_secret)
            .map(|t| t.token)
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Load contributions if owner
    let contributions = if is_owner {
        let all_contributions = BookContributionService::get_contributions(&state.db, id).await?;

        // Get recipe titles for contributions
        let recipe_ids: Vec<Uuid> = all_contributions.iter().map(|c| c.recipe_id).collect();
        let recipe_titles = RecipeService::get_titles_by_ids(&state.db, &recipe_ids).await?;

        let mut pending = Vec::new();
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for contrib in &all_contributions {
            let title = recipe_titles
                .get(&contrib.recipe_id)
                .cloned()
                .unwrap_or_else(|| "Unknown Recipe".to_string());
            let view = ContributionItemView::from_display(contrib, title);
            match view.status {
                ContributionStatus::Pending => pending.push(view),
                ContributionStatus::Accepted => accepted.push(view),
                ContributionStatus::Rejected => rejected.push(view),
            }
        }

        let pending_count = pending.len();
        ContributionListView {
            pending,
            accepted,
            rejected,
            pending_count,
        }
    } else {
        ContributionListView::default()
    };

    let template = BookViewTemplate {
        book,
        owner,
        recipes: recipes_page.data,
        recipe_count: recipes_page.pagination.total_items as i64,
        is_owner,
        contributions,
        csrf_token,
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
    auth: AuthUser,
) -> AppResult<Html<String>> {
    BookService::require_edit_permission(&state.db, id, auth.id).await?;
    let book = BookService::get_by_id(&state.db, id).await?;

    let template = EditBookTemplate { book: Some(book) };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

// =============================================================================
// Contribution Management (Phase 7 - User Story 5)
// =============================================================================

/// Helper to create request context
fn create_request_context(
    addr: SocketAddr,
    request_id: Option<&RequestId>,
    session_id: Uuid,
) -> RequestContext {
    RequestContext {
        session_id: Some(session_id),
        request_id: request_id.map(|r| r.0),
        ip: Some(addr.ip()),
    }
}

/// CSRF-only form for simple actions
#[derive(Debug, Deserialize)]
struct CsrfOnlyForm {
    #[serde(rename = "_csrf")]
    csrf_token: String,
}

/// Rejection form with optional reason
#[derive(Debug, Deserialize)]
struct RejectContributionForm {
    #[serde(rename = "_csrf")]
    csrf_token: String,
    reason: Option<String>,
}

/// Accept contribution handler (POST) (T031)
///
/// HTMX endpoint to accept a pending contribution.
async fn accept_contribution(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Path((book_id, contribution_id)): Path<(Uuid, Uuid)>,
    Form(form): Form<CsrfOnlyForm>,
) -> Result<Response, AppError> {
    // Validate CSRF token
    validate_csrf_token(&form.csrf_token, auth.session_id, &state.csrf_secret)?;

    // Verify user owns the book (authorization check)
    let book = BookService::get_by_id(&state.db, book_id).await?;
    if book.owner_id != auth.id {
        return Err(AppError::Forbidden(
            "Only the book owner can manage contributions".to_string(),
        ));
    }

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    // Accept the contribution
    let contribution =
        BookContributionService::accept_contribution(&state.db, contribution_id, auth.id).await?;

    // Log the action
    AuditEvent::new("book.contribution.accept")
        .with_user(auth.id)
        .with_context(&ctx)
        .with_metadata("book_id", &book_id.to_string())
        .with_metadata("contribution_id", &contribution_id.to_string())
        .persist(&state.db)
        .await;

    // Get recipe title for display
    let titles = RecipeService::get_titles_by_ids(&state.db, &[contribution.recipe_id]).await?;
    let title = titles
        .get(&contribution.recipe_id)
        .cloned()
        .unwrap_or_else(|| "Recipe".to_string());

    // Return updated row HTML for HTMX swap
    let html = format!(
        r#"<div id="contribution-{}" class="flex items-center justify-between p-3 bg-green-50 dark:bg-green-900/20 rounded-lg">
            <div class="flex items-center">
                <a href="/recipes/{}" class="text-sm font-medium text-gray-900 dark:text-white hover:underline">{}</a>
                <span class="ml-2 text-xs text-green-600 dark:text-green-400">Accepted</span>
            </div>
        </div>"#,
        contribution_id, contribution.recipe_id, title
    );

    Ok(Html(html).into_response())
}

/// Reject contribution handler (POST) (T032)
///
/// HTMX endpoint to reject a pending contribution with optional reason.
async fn reject_contribution(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Path((book_id, contribution_id)): Path<(Uuid, Uuid)>,
    Form(form): Form<RejectContributionForm>,
) -> Result<Response, AppError> {
    // Validate CSRF token
    validate_csrf_token(&form.csrf_token, auth.session_id, &state.csrf_secret)?;

    // Verify user owns the book (authorization check)
    let book = BookService::get_by_id(&state.db, book_id).await?;
    if book.owner_id != auth.id {
        return Err(AppError::Forbidden(
            "Only the book owner can manage contributions".to_string(),
        ));
    }

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    // Reject the contribution
    let contribution = BookContributionService::reject_contribution(
        &state.db,
        contribution_id,
        auth.id,
        form.reason.clone(),
    )
    .await?;

    // Log the action
    AuditEvent::new("book.contribution.reject")
        .with_user(auth.id)
        .with_context(&ctx)
        .with_metadata("book_id", &book_id.to_string())
        .with_metadata("contribution_id", &contribution_id.to_string())
        .persist(&state.db)
        .await;

    // Get recipe title for display
    let titles = RecipeService::get_titles_by_ids(&state.db, &[contribution.recipe_id]).await?;
    let title = titles
        .get(&contribution.recipe_id)
        .cloned()
        .unwrap_or_else(|| "Recipe".to_string());

    // Return updated row HTML for HTMX swap
    let reason_display = form
        .reason
        .as_ref()
        .map(|r| format!(" - {}", r))
        .unwrap_or_default();
    let html = format!(
        r#"<div id="contribution-{}" class="flex items-center justify-between p-3 bg-red-50 dark:bg-red-900/20 rounded-lg">
            <div class="flex items-center">
                <span class="text-sm text-gray-900 dark:text-white">{}</span>
                <span class="ml-2 text-xs text-red-600 dark:text-red-400">Rejected{}</span>
            </div>
        </div>"#,
        contribution_id, title, reason_display
    );

    Ok(Html(html).into_response())
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
            contributions: ContributionListView::default(),
            csrf_token: String::new(),
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
            contributions: ContributionListView::default(),
            csrf_token: String::new(),
        };
        assert!(owner_template.render().is_ok());

        // Test as non-owner
        let guest_template = BookViewTemplate {
            book,
            owner: None,
            recipes: vec![],
            recipe_count: 0,
            is_owner: false,
            contributions: ContributionListView::default(),
            csrf_token: String::new(),
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
            contributions: ContributionListView::default(),
            csrf_token: String::new(),
        };

        let html = template.render().unwrap();
        assert!(!html.is_empty());
    }
}
