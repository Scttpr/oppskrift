//! Authorized-resource extractor — load a resource and its permission in one crossing.
//!
//! Builds on [`Viewer`]: resolves the signed-in user, evaluates the permission
//! once via [`PermissionService`], then loads the resource row. Replaces the
//! `get_by_id_authorized(...)` + hand-rolled `is_owner = viewer_id == owner` and
//! `require_edit_permission(...)` patterns repeated across handlers, returning a
//! value that already encodes "you may see/edit this" (404, not 403, when not).

use std::future::Future;
use std::marker::PhantomData;

use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{AuthUser, OptionalViewer, Viewer};
use crate::core::error::AppResult;
use crate::models::{PermissionLevel, Recipe, RecipeBook, ResourceType};
use crate::services::{BookService, PermissionService, RecipeService};

/// A resource that can be loaded and permission-checked by id.
pub trait AuthzResource: Sized + Send {
    /// The permission domain this resource belongs to.
    const RESOURCE_TYPE: ResourceType;
    /// The owner of this resource (full access, basis for `is_owner`).
    fn owner_id(&self) -> Uuid;
    /// Load the full resource row, 404 when it does not exist.
    fn load(db: &PgPool, id: Uuid) -> impl Future<Output = AppResult<Self>> + Send;
}

impl AuthzResource for Recipe {
    const RESOURCE_TYPE: ResourceType = ResourceType::Recipe;

    fn owner_id(&self) -> Uuid {
        self.author_id
    }

    async fn load(db: &PgPool, id: Uuid) -> AppResult<Self> {
        RecipeService::get_by_id(db, id).await
    }
}

impl AuthzResource for RecipeBook {
    const RESOURCE_TYPE: ResourceType = ResourceType::Book;

    fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    async fn load(db: &PgPool, id: Uuid) -> AppResult<Self> {
        BookService::get_by_id(db, id).await
    }
}

/// The permission level an [`Authorized`] extractor requires.
pub trait AuthzLevel {
    /// Level demanded of the viewer before the handler runs.
    const LEVEL: PermissionLevel;
    /// Whether an unauthenticated viewer is rejected up front (login redirect)
    /// rather than being told the resource does not exist.
    const REQUIRE_AUTH: bool;
}

/// Read access — anonymous viewers are allowed (public/followers resources).
pub struct View;
impl AuthzLevel for View {
    const LEVEL: PermissionLevel = PermissionLevel::View;
    const REQUIRE_AUTH: bool = false;
}

/// Edit access — anonymous viewers get the standard auth rejection.
pub struct Edit;
impl AuthzLevel for Edit {
    const LEVEL: PermissionLevel = PermissionLevel::Edit;
    const REQUIRE_AUTH: bool = true;
}

/// A resource the viewer is already authorized to access at level `L`.
///
/// Loading, the permission check, and ownership are decided at this one seam;
/// handlers receive a resource they may act on plus the resolved [`Viewer`].
pub struct Authorized<R, L> {
    /// The loaded resource.
    pub resource: R,
    /// The signed-in viewer, `None` for an authorized anonymous (view) request.
    pub viewer: Option<Viewer>,
    /// The viewer's effective level on the resource.
    pub effective_level: Option<PermissionLevel>,
    /// Whether the viewer owns the resource.
    pub is_owner: bool,
    _level: PhantomData<L>,
}

impl<R, L> Authorized<R, L> {
    /// The viewer's id, `None` when anonymous.
    pub fn viewer_id(&self) -> Option<Uuid> {
        self.viewer.as_ref().map(|v| v.id)
    }
}

impl<R, L> FromRequestParts<crate::AppState> for Authorized<R, L>
where
    R: AuthzResource,
    L: AuthzLevel,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        let OptionalViewer(viewer) = OptionalViewer::from_request_parts(parts, state)
            .await
            .unwrap_or(OptionalViewer(None));

        // Edit-style levels redirect an anonymous visitor to sign in rather than
        // hiding the resource behind a 404.
        if L::REQUIRE_AUTH && viewer.is_none() {
            AuthUser::from_request_parts(parts, state).await?;
        }

        let Path(id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;

        let viewer_id = viewer.as_ref().map(|v| v.id);
        let check = PermissionService::require_permission(
            &state.db,
            viewer_id,
            R::RESOURCE_TYPE,
            id,
            L::LEVEL,
        )
        .await
        .map_err(IntoResponse::into_response)?;

        let resource = R::load(&state.db, id)
            .await
            .map_err(IntoResponse::into_response)?;
        let is_owner = viewer_id == Some(resource.owner_id());

        Ok(Authorized {
            resource,
            viewer,
            effective_level: check.effective_level,
            is_owner,
            _level: PhantomData,
        })
    }
}
