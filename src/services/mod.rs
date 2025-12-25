// Services module - RecipeService, BookService, ActivityService, FederationService

pub mod recipe_service;
pub mod user_service;

pub use recipe_service::RecipeService;
pub use user_service::UserService;
