use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};
use uuid::Uuid;

use crate::api::middleware::OptionalAuthUser;
use crate::lib::error::AppResult;
use crate::lib::pagination::{PaginationMeta, PaginationParams};
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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}
