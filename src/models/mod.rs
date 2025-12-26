// Models module - Recipe, RecipeBook, Ingredient, User, Activity, Social, Auth

#![allow(dead_code)]
// Allow unused imports for auth models until API endpoints are implemented
#![allow(unused_imports)]

pub mod activity;
pub mod auth;
pub mod book_recipe_entry;
pub mod email_confirmation;
pub mod follow;
pub mod ingredient;
pub mod instruction_step;
pub mod recipe;
pub mod recipe_book;
pub mod recipe_image;
pub mod saved_recipe;
pub mod security_event;
pub mod session;
pub mod user;

pub use activity::{Activity, ActivityType, ActivityWithActor, CreateActivity, TargetType};
pub use auth::{
    AuthError, LoginRequest, LoginResponse, LogoutResponse, RegisterRequest, RegisterResponse,
    TwoFactorRequired,
};
pub use book_recipe_entry::{AddRecipeToBook, BookRecipeEntry};
pub use email_confirmation::{
    ConfirmEmailRequest, CreateEmailConfirmationToken, EmailConfirmationInfo,
    EmailConfirmationResponse, EmailConfirmationToken, ResendConfirmationRequest,
};
pub use follow::{Follow, FollowCounts};
pub use ingredient::{CreateIngredient, Ingredient};
pub use instruction_step::{CreateInstructionStep, InstructionStep};
pub use recipe::{CreateRecipe, Difficulty, Recipe, RecipeSummary, UpdateRecipe, Visibility};
pub use recipe_book::{CreateRecipeBook, RecipeBook, RecipeBookSummary, UpdateRecipeBook};
pub use recipe_image::RecipeImage;
pub use saved_recipe::SavedRecipe;
pub use security_event::{SecurityEvent, SecurityEventInfo, SecurityEventsResponse};
pub use session::{
    CreateSession, RevokeAllSessionsResponse, RevokeSessionRequest, Session, SessionInfo,
    SessionListResponse,
};
pub use user::{CreateUser, MeasurementPref, User, UserProfile, RESERVED_USERNAMES};
