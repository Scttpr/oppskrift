# Tasks: ABAC Authorization System

**Input**: Design documents from `/specs/005-abac-authorization/`
**Prerequisites**: plan.md, spec.md, data-model.md, contracts/, research.md, quickstart.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Exact file paths included in descriptions

---

## Phase 1: Setup (Shared Infrastructure) ✅

**Purpose**: Database migrations and enum extensions

- [x] T001 Create migration for followers_only visibility in migrations/20251230000001_add_visibility_followers_only.sql
- [x] T002 [P] Create migration for groups table in migrations/20251230000002_create_groups_table.sql
- [x] T003 [P] Create migration for group_members table in migrations/20251230000003_create_group_members_table.sql
- [x] T004 [P] Create migration for permissions table in migrations/20251230000004_create_permissions_table.sql
- [x] T005 [P] Create migration for book_contributions table in migrations/20251230000005_create_book_contributions_table.sql
- [x] T006 [P] Create migration for permission_audit_log table in migrations/20251230000006_create_permission_audit_log.sql
- [x] T007 Run migrations and regenerate SQLx offline cache with `cargo sqlx prepare`

---

## Phase 2: Foundational (Blocking Prerequisites) ✅

**Purpose**: Core models and permission service that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T008 Extend Visibility enum with FollowersOnly variant in src/models/recipe.rs
- [x] T009 [P] Create PermissionLevel enum (view, edit, contributor) in src/models/permission.rs
- [x] T010 [P] Create SubjectType enum (user, group, instance) in src/models/permission.rs
- [x] T011 Create Permission model with SQLx mappings in src/models/permission.rs
- [x] T012 [P] Create Group model in src/models/group.rs
- [x] T013 [P] Create GroupMember model in src/models/group.rs
- [x] T014 [P] Create BookContribution model in src/models/book_contribution.rs
- [x] T015 [P] Create PermissionAuditLog model in src/models/audit.rs
- [x] T016 Update src/models/mod.rs to export new modules
- [x] T017 Create PermissionService with check_permission() core logic in src/services/permission_service.rs
- [x] T018 Implement permission evaluation order (owner → direct → group → followers → public) in src/services/permission_service.rs
- [x] T019 Implement highest-permission-wins logic for multiple paths in src/services/permission_service.rs
- [x] T020 Create audit logging helper in src/services/permission_service.rs
- [x] T021 Update src/services/mod.rs to export permission_service

**Checkpoint**: Foundation ready - user story implementation can now begin ✅

---

## Phase 3: User Story 1 - Owner Full Control (Priority: P1) 🎯 MVP ✅

**Goal**: Resource owners have full control (view, edit, delete, share) over their content

**Independent Test**: Create a recipe/book as user, verify CRUD works only for owner, non-owners get 404

### Implementation for User Story 1

- [x] T022 [US1] Update RecipeService.get_by_id_authorized() to use PermissionService in src/services/recipe_service.rs
- [x] T023 [US1] Update BookService.get_by_id_authorized() to use PermissionService in src/services/book_service.rs
- [x] T024 [US1] Update recipe update/delete handlers to use PermissionService check in src/api/recipes.rs
- [x] T025 [US1] Update book update/delete handlers to use PermissionService check in src/api/books.rs
- [x] T026 [US1] Ensure 404 response for unauthorized access (not 403) in src/services/permission_service.rs
- [x] T027 [US1] Add owner check as first short-circuit in permission evaluation in src/services/permission_service.rs

**Checkpoint**: Owner full control works - owners can CRUD, non-owners get 404 ✅

---

## Phase 4: User Story 2 - Public Sharing (Priority: P1) ✅

**Goal**: Public visibility makes content accessible to anyone including unauthenticated users

**Independent Test**: Set recipe to public, verify unauthenticated users can view it

### Implementation for User Story 2

- [x] T028 [US2] Ensure public visibility check is last in permission chain in src/services/permission_service.rs
- [x] T029 [US2] Update recipe list endpoint to filter by visibility for non-owners in src/api/recipes.rs
- [x] T030 [US2] Update book list endpoint to filter by visibility for non-owners in src/api/books.rs
- [x] T031 [US2] Verify OptionalAuthUser extractor passes for public resources in src/api/middleware/auth.rs

**Checkpoint**: Public sharing works - public content visible to all ✅

---

## Phase 5: User Story 3 - Private Content (Priority: P1) ✅

**Goal**: Private visibility restricts access to owner only (and explicit shares)

**Independent Test**: Create private recipe, verify other users get 404

### Implementation for User Story 3

- [x] T032 [US3] Ensure default visibility is Private for new recipes in src/services/recipe_service.rs
- [x] T033 [US3] Ensure default visibility is Private for new books in src/services/book_service.rs
- [x] T034 [US3] Verify private resources excluded from public lists in src/api/recipes.rs
- [x] T035 [US3] Verify private books excluded from public lists in src/api/books.rs

**Checkpoint**: Private content works - private resources invisible to non-owners ✅

---

## Phase 6: User Story 4 - Share with Specific Users (Priority: P2) ✅

**Goal**: Share resources with specific users at view or edit permission levels

**Independent Test**: Share private recipe with user, verify they can access, others cannot

### Implementation for User Story 4

- [x] T036 [US4] Implement grant_permission() in src/services/permission_service.rs
- [x] T037 [US4] Implement revoke_permission() in src/services/permission_service.rs
- [x] T038 [US4] Implement list_permissions() for resource in src/services/permission_service.rs
- [x] T039 [US4] Implement direct-share check in permission evaluation in src/services/permission_service.rs
- [x] T040 [P] [US4] Create POST /api/v1/recipes/{id}/permissions endpoint in src/api/recipes.rs
- [x] T041 [P] [US4] Create DELETE /api/v1/recipes/{id}/permissions/{perm_id} endpoint in src/api/recipes.rs
- [x] T042 [P] [US4] Create GET /api/v1/recipes/{id}/permissions endpoint in src/api/recipes.rs
- [x] T043 [P] [US4] Create POST /api/v1/books/{id}/permissions endpoint in src/api/books.rs
- [x] T044 [P] [US4] Create DELETE /api/v1/books/{id}/permissions/{perm_id} endpoint in src/api/books.rs
- [x] T045 [P] [US4] Create GET /api/v1/books/{id}/permissions endpoint in src/api/books.rs
- [x] T046 [US4] Update src/api/mod.rs to register permission routes
- [x] T047 [US4] Log permission changes to audit log in src/services/permission_service.rs

**Checkpoint**: User sharing works - share with specific users, revoke access ✅

---

## Phase 7: User Story 5 - Followers Access (Priority: P2) ✅

**Goal**: Followers-only visibility grants access to all followers

**Independent Test**: Set recipe to followers-only, verify follower can access, non-follower cannot

### Implementation for User Story 5

- [x] T048 [US5] Update Recipe model to support followers_only visibility in src/models/recipe.rs
- [x] T049 [US5] Update RecipeBook model to support followers_only visibility in src/models/recipe_book.rs
- [x] T050 [US5] Implement followers check in permission evaluation (after group, before public) in src/services/permission_service.rs
- [x] T051 [US5] Query follows table to check if user follows resource owner in src/services/permission_service.rs
- [x] T052 [US5] Add visibility update endpoint to allow setting followers_only in src/api/recipes.rs
- [x] T053 [US5] Add visibility update endpoint to allow setting followers_only in src/api/books.rs

**Checkpoint**: Followers-only works - followers can access, non-followers get 404 ✅

---

## Phase 8: User Story 6 - Share with Groups (Priority: P3) ✅

**Goal**: Create groups and share content with entire groups

**Independent Test**: Create group, add member, share recipe with group, member can access

### Implementation for User Story 6

- [x] T054 [US6] Create GroupService with CRUD operations in src/services/group_service.rs
- [x] T055 [US6] Implement add_member() in src/services/group_service.rs
- [x] T056 [US6] Implement remove_member() in src/services/group_service.rs
- [x] T057 [US6] Implement get_user_groups() in src/services/group_service.rs
- [x] T058 [US6] Implement group-share check in permission evaluation in src/services/permission_service.rs
- [x] T059 [P] [US6] Create POST /api/v1/groups endpoint in src/api/groups.rs
- [x] T060 [P] [US6] Create GET /api/v1/groups endpoint in src/api/groups.rs
- [x] T061 [P] [US6] Create GET /api/v1/groups/{id} endpoint in src/api/groups.rs
- [x] T062 [P] [US6] Create PUT /api/v1/groups/{id} endpoint in src/api/groups.rs
- [x] T063 [P] [US6] Create DELETE /api/v1/groups/{id} endpoint in src/api/groups.rs
- [x] T064 [P] [US6] Create POST /api/v1/groups/{id}/members endpoint in src/api/groups.rs
- [x] T065 [P] [US6] Create DELETE /api/v1/groups/{id}/members/{user_id} endpoint in src/api/groups.rs
- [x] T066 [P] [US6] Create POST /api/v1/groups/{id}/leave endpoint in src/api/groups.rs
- [x] T067 [US6] Update src/api/mod.rs to register group routes
- [x] T068 [US6] Update src/services/mod.rs to export group_service
- [x] T069 [US6] Group permission check uses JOIN query (no materialized view needed)

**Checkpoint**: Group sharing works - share with groups, members get access ✅

---

## Phase 9: User Story 7 - Share with Federated Instances (Priority: P3) ✅

**Goal**: Share content with all users from a specific federated instance

**Independent Test**: Share recipe with instance domain, user from that instance can access

### Implementation for User Story 7

- [x] T070 [US7] Implement instance-share check in permission evaluation in src/services/permission_service.rs
- [x] T071 [US7] Extract instance domain from federated user actor URL in src/services/permission_service.rs
- [x] T072 [US7] Add instance domain parameter to grant_permission() in src/services/permission_service.rs
- [x] T073 [US7] Update permission endpoints to accept instance shares in src/api/recipes.rs and src/api/books.rs

**Checkpoint**: Instance sharing works - federated instance users get access ✅

---

## Phase 10: User Story 8 - Collaborative Book Editing (Priority: P2) ✅

**Goal**: Contributors can add their own recipes to a book while maintaining ownership

**Independent Test**: Grant contributor access, contributor adds recipe, recipe stays theirs

### Implementation for User Story 8

- [x] T074 [US8] Create BookContributionService in src/services/book_contribution_service.rs
- [x] T075 [US8] Implement add_contribution() with ownership validation in src/services/book_contribution_service.rs
- [x] T076 [US8] Implement remove_contribution() (by owner or contributor) in src/services/book_contribution_service.rs
- [x] T077 [US8] Implement get_book_contributions() in src/services/book_contribution_service.rs
- [x] T078 [US8] Validate contributor permission before allowing add in src/services/book_contribution_service.rs
- [x] T079 [P] [US8] Create POST /api/v1/books/{id}/contributions endpoint in src/api/books.rs
- [x] T080 [P] [US8] Create DELETE /api/v1/books/{id}/contributions/{recipe_id} endpoint in src/api/books.rs
- [x] T081 [US8] Update book recipes list to include contributed recipes in src/services/book_service.rs
- [x] T082 [US8] Ensure contributed recipe ownership unchanged in src/services/book_contribution_service.rs
- [x] T083 [US8] Update src/services/mod.rs to export book_contribution_service

**Checkpoint**: Collaborative books work - contributors add recipes, keep ownership ✅

---

## Phase 11: User Story 9 - Permission Management Interface (Priority: P2) ✅

**Goal**: UI for viewing and managing permissions on resources

**Independent Test**: Navigate to recipe sharing settings, view/add/remove permissions

### Implementation for User Story 9

- [x] T084 [P] [US9] Create permissions management handler in src/handlers/permissions.rs
- [x] T085 [P] [US9] Create share settings page template in templates/permissions/manage.html
- [x] T086 [US9] Permission list integrated into manage.html (no separate component needed)
- [x] T087 [US9] User search integrated into manage.html (no separate component needed)
- [x] T088 [P] [US9] Create group list template in templates/groups/list.html
- [x] T089 [P] [US9] Create group detail template in templates/groups/view.html
- [x] T090 [P] [US9] Create group create/edit form template in templates/groups/form.html
- [x] T091 [P] [US9] Create groups handler in src/handlers/groups.rs
- [x] T092 [US9] Update src/handlers/mod.rs to register new handlers
- [x] T093 [US9] Share button inline in recipe/book templates (no separate component needed)
- [x] T094 [US9] Integrate share button into recipe detail template in templates/recipes/view.html
- [x] T095 [US9] Integrate share button into book detail template in templates/books/view.html

**Checkpoint**: Permission UI works - users can manage sharing through interface ✅

---

## Phase 12: Polish & Cross-Cutting Concerns ✅

**Purpose**: Performance, security, and code quality improvements

- [x] T096 [P] Document caching strategy in src/services/permission_service.rs (implementation deferred - DB queries are fast)
- [x] T097 [P] Cache invalidation documented for future use in src/services/permission_service.rs
- [x] T098 [P] Add privilege escalation prevention (can_grant_level()) in src/services/permission_service.rs
- [x] T099 [P] Add cleanup for orphaned permissions (cleanup_*_permissions methods) in src/services/permission_service.rs
- [x] T100 [P] Add test helpers for permissions in tests/common/mod.rs
- [x] T101 [P] Create authorization integration tests (14 tests) in tests/authorization_test.rs
- [x] T102 Update SQLx offline cache with `cargo sqlx prepare`
- [x] T103 Run full test suite - all 197 tests pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Stories (Phases 3-11)**: All depend on Foundational completion
  - US1-3 (P1): Can proceed in priority order
  - US4-5, US8-9 (P2): Can proceed after P1 or in parallel if staffed
  - US6-7 (P3): Can proceed after P2 or in parallel if staffed
- **Polish (Phase 12)**: After all user stories complete

### User Story Dependencies

- **US1 (Owner Control)**: Foundation only - no story dependencies
- **US2 (Public)**: Foundation only - no story dependencies
- **US3 (Private)**: Foundation only - no story dependencies
- **US4 (User Sharing)**: Foundation only - enables US6, US7
- **US5 (Followers)**: Foundation only - no story dependencies
- **US6 (Groups)**: Benefits from US4 patterns
- **US7 (Instances)**: Benefits from US4 patterns
- **US8 (Contributions)**: Foundation only - no story dependencies
- **US9 (UI)**: Benefits from US4, US6 being complete

### Parallel Opportunities

Within Phase 2 (Foundational):
- T009, T010, T012, T013, T014, T015 can run in parallel

Within US4 (User Sharing):
- T040-T045 can run in parallel (different endpoints)

Within US6 (Groups):
- T059-T066 can run in parallel (different endpoints)

Within US9 (UI):
- T084-T091 can run in parallel (different files)

---

## Implementation Strategy

### MVP First (US1 + US2 + US3)

1. Complete Phase 1: Setup (migrations)
2. Complete Phase 2: Foundational (models, permission service)
3. Complete Phase 3: US1 - Owner Control
4. Complete Phase 4: US2 - Public Sharing
5. Complete Phase 5: US3 - Private Content
6. **STOP and VALIDATE**: Basic authorization works
7. Deploy/demo MVP

### Incremental Delivery

1. MVP (US1-3) → Test → Deploy
2. Add US4 (User Sharing) → Test → Deploy
3. Add US5 (Followers) + US8 (Contributions) → Test → Deploy
4. Add US6 (Groups) + US9 (UI) → Test → Deploy
5. Add US7 (Instances) → Test → Deploy
6. Polish phase → Final release

---

## Notes

- All unauthorized access returns 404 (not 403) per clarifications
- Permissions are permanent until explicitly revoked
- Book sharing cascades to contained recipes
- Contributor level only applies to books
- Recipe ownership unchanged when added to collaborative books
