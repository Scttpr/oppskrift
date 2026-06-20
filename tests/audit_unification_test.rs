//! Integration tests for the unified audit seam (C4).
//!
//! Permission and access events now cross the single `AuditEvent` interface,
//! which routes them into `permission_audit_log`. These tests drive the public
//! API and assert the relational forensic row is written with the right
//! actor/resource/subject/level columns — the behaviour that previously lived
//! in `PermissionService::log_audit` + `CreateAuditLog`.

mod common;

use common::run_test;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct AuditRow {
    event_type: String,
    actor_id: Option<Uuid>,
    resource_type: Option<String>,
    resource_id: Option<Uuid>,
    subject_type: Option<String>,
    subject_id: Option<Uuid>,
    permission_level: Option<String>,
}

async fn latest_audit(db: &sqlx::PgPool, event_type: &str, resource_id: Uuid) -> Option<AuditRow> {
    sqlx::query_as::<_, AuditRow>(
        r#"
        SELECT event_type, actor_id, resource_type, resource_id,
               subject_type, subject_id, permission_level
        FROM permission_audit_log
        WHERE event_type = $1 AND resource_id = $2
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .bind(event_type)
    .bind(resource_id)
    .fetch_optional(db)
    .await
    .expect("query permission_audit_log")
}

#[tokio::test]
async fn test_permission_granted_writes_audit_row() {
    run_test(|mut ctx| async move {
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;
        let (grantee_id, _grantee_session) = ctx.create_and_login("grantee").await;
        let recipe_id = ctx
            .create_recipe(owner_id, "Audited Recipe", "private")
            .await;

        let grant = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": grantee_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;
        assert!(grant.is_success(), "grant should succeed");

        let row = latest_audit(&ctx.db, "permission_granted", recipe_id)
            .await
            .expect("a permission_granted row should be written");
        assert_eq!(row.event_type, "permission_granted");
        assert_eq!(row.actor_id, Some(owner_id));
        assert_eq!(row.resource_type.as_deref(), Some("recipe"));
        assert_eq!(row.resource_id, Some(recipe_id));
        assert_eq!(row.subject_type.as_deref(), Some("user"));
        assert_eq!(row.subject_id, Some(grantee_id));
        assert_eq!(row.permission_level.as_deref(), Some("view"));
    })
    .await;
}

#[tokio::test]
async fn test_permission_revoked_writes_audit_row() {
    run_test(|mut ctx| async move {
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;
        let (grantee_id, _grantee_session) = ctx.create_and_login("grantee").await;
        let recipe_id = ctx
            .create_recipe(owner_id, "Audited Recipe", "private")
            .await;

        let grant = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": grantee_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;
        let permission_id = grant
            .body
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Uuid>().ok())
            .expect("grant response carries the new permission id");

        let revoke = ctx
            .delete_with_session(
                &format!(
                    "/api/v1/recipes/{}/permissions/{}",
                    recipe_id, permission_id
                ),
                &owner_session,
            )
            .await;
        assert!(revoke.is_success(), "revoke should succeed");

        let row = latest_audit(&ctx.db, "permission_revoked", recipe_id)
            .await
            .expect("a permission_revoked row should be written");
        assert_eq!(row.actor_id, Some(owner_id));
        assert_eq!(row.subject_id, Some(grantee_id));
        assert_eq!(row.permission_level.as_deref(), Some("view"));
    })
    .await;
}

#[tokio::test]
async fn test_access_denied_writes_audit_row() {
    run_test(|mut ctx| async move {
        let (owner_id, _owner_session) = ctx.create_and_login("owner").await;
        let (_other_id, other_session) = ctx.create_and_login("other").await;
        let recipe_id = ctx
            .create_recipe(owner_id, "Audited Recipe", "private")
            .await;

        // A non-owner viewing a private recipe is denied (404), which audits.
        let denied = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &other_session)
            .await;
        assert_eq!(denied.status, 404, "non-owner view must be denied");

        let row = latest_audit(&ctx.db, "access_denied", recipe_id)
            .await
            .expect("an access_denied row should be written");
        assert_eq!(row.resource_type.as_deref(), Some("recipe"));
        assert_eq!(row.resource_id, Some(recipe_id));
    })
    .await;
}

/// The grant and its audit entry share one transaction: if the audit write
/// fails, the grant must roll back. We force the audit insert to fail with a
/// `BEFORE INSERT` trigger scoped to this test's unique `resource_id` (so it
/// never touches other tests' audit writes), then assert no permission row
/// survived.
#[tokio::test]
async fn test_grant_rolls_back_when_audit_write_fails() {
    run_test(|mut ctx| async move {
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;
        let (grantee_id, _grantee_session) = ctx.create_and_login("grantee").await;
        let recipe_id = ctx
            .create_recipe(owner_id, "Audited Recipe", "private")
            .await;

        // Fail only the audit insert for this recipe's grants.
        let trigger = format!("test_fail_audit_{}", recipe_id.simple());
        sqlx::query(&format!(
            r#"
            CREATE OR REPLACE FUNCTION {trigger}() RETURNS trigger AS $$
            BEGIN
                IF NEW.resource_id = '{recipe_id}'::uuid THEN
                    RAISE EXCEPTION 'injected audit failure';
                END IF;
                RETURN NEW;
            END;
            $$ LANGUAGE plpgsql;
            "#
        ))
        .execute(&ctx.db)
        .await
        .expect("create trigger function");
        sqlx::query(&format!(
            "CREATE TRIGGER {trigger} BEFORE INSERT ON permission_audit_log \
             FOR EACH ROW EXECUTE FUNCTION {trigger}()"
        ))
        .execute(&ctx.db)
        .await
        .expect("create trigger");

        let grant = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": grantee_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;
        assert!(
            !grant.is_success(),
            "grant must fail when its audit write fails, got {}",
            grant.status
        );

        // Rollback: neither the grant nor the audit row may survive.
        let perms: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM permissions WHERE resource_id = $1")
                .bind(recipe_id)
                .fetch_one(&ctx.db)
                .await
                .expect("count permissions");
        assert_eq!(perms, 0, "the grant must have rolled back");

        let audits: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM permission_audit_log WHERE resource_id = $1")
                .bind(recipe_id)
                .fetch_one(&ctx.db)
                .await
                .expect("count audit rows");
        assert_eq!(audits, 0, "no audit row should be committed");

        // Cleanup the injected trigger.
        sqlx::query(&format!(
            "DROP TRIGGER IF EXISTS {trigger} ON permission_audit_log"
        ))
        .execute(&ctx.db)
        .await
        .ok();
        sqlx::query(&format!("DROP FUNCTION IF EXISTS {trigger}()"))
            .execute(&ctx.db)
            .await
            .ok();
    })
    .await;
}
