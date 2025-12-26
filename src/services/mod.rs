// Services module - RecipeService, BookService, ActivityService, SocialService, Auth services

#![allow(dead_code)]
// Allow unused imports for auth services until API endpoints are implemented
#![allow(unused_imports)]

pub mod activity_service;
pub mod book_service;
pub mod email_service;
pub mod follow_service;
pub mod image_service;
pub mod password_service;
pub mod recipe_service;
pub mod saved_recipe_service;
pub mod security_log_service;
pub mod session_service;
pub mod user_service;

pub use activity_service::ActivityService;
pub use book_service::BookService;
pub use email_service::EmailService;
pub use follow_service::FollowService;
pub use image_service::ImageService;
pub use password_service::PasswordService;
pub use recipe_service::RecipeService;
pub use saved_recipe_service::SavedRecipeService;
pub use security_log_service::{SecurityEventBuilder, SecurityEventType, SecurityLogService};
pub use session_service::SessionService;
pub use user_service::UserService;
