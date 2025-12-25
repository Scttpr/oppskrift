# Tasks: Recipe Creation and Sharing

**Input**: Design documents from `/specs/001-recipe-sharing/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/openapi.yaml

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md project structure:
- `src/` - Rust source code (models, services, api, handlers, lib)
- `templates/` - Askama HTML templates
- `static/` - CSS, JS (HTMX)
- `tests/` - Integration tests
- `migrations/` - SQLx database migrations

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, tooling, and basic structure

- [x] T001 Initialize Rust project with `cargo init` and configure Cargo.toml with dependencies from research.md
- [x] T002 Create project directory structure: src/{models,services,api,handlers,lib}, templates/, static/, tests/, migrations/
- [x] T003 [P] Configure rustfmt.toml and clippy.toml for code formatting/linting
- [x] T004 [P] Create .env.example with required environment variables (DATABASE_URL, S3_*, etc.)
- [x] T005 [P] Download and vendor HTMX to static/js/htmx.min.js
- [x] T006 [P] Download Tailwind standalone CLI binary, add to .gitignore
- [x] T007 [P] Create tailwind.config.js with theme colors, dark mode (class strategy), and content paths
- [x] T008 [P] Create static/css/input.css with Tailwind directives and CSS custom properties for instance theming
- [x] T009 Add Tailwind build command to Makefile (`./tailwindcss -i static/css/input.css -o static/css/main.css --minify`)
- [x] T010 Create Dockerfile and docker-compose.yml for development (app + PostgreSQL)
- [x] T011 [P] Add AGPL-3.0 LICENSE file to repository root
- [x] T012 [P] Configure cargo-audit in CI workflow for dependency vulnerability scanning

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T013 Setup SQLx with PostgreSQL connection pool in src/lib/db.rs
- [x] T014 Create database migration for enums (visibility_type, difficulty_type, measurement_pref, activity_type, target_type) in migrations/001_enums.sql
- [x] T015 Create User table migration in migrations/002_users.sql
- [x] T016 Implement User model in src/models/user.rs with SQLx derives
- [x] T017 [P] Setup Axum router skeleton in src/main.rs with tower-http middleware (CORS, tracing)
- [x] T018 [P] Create error types in src/lib/error.rs with thiserror
- [x] T019 [P] Create pagination types in src/lib/pagination.rs
- [x] T020 Create base Askama layout template in templates/layouts/base.html with HTMX script, Tailwind CSS, skip links, and ARIA landmarks (main, nav, banner, contentinfo)
- [x] T021 [P] Setup static file serving for /static/* in src/main.rs
- [x] T022 Implement auth middleware stub in src/api/middleware/auth.rs (extract user from JWT, placeholder for external auth)
- [x] T023 Create UserService in src/services/user_service.rs with get_by_id, get_profile methods
- [x] T024 Implement GET /api/v1/users/{id} endpoint in src/api/users.rs
- [x] T025 Implement GET /api/v1/users/me endpoint in src/api/users.rs
- [x] T026 Implement PATCH /api/v1/users/me endpoint for preferences update in src/api/users.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Create a Recipe (Priority: P1) 🎯 MVP

**Goal**: Users can create, view, edit, and delete recipes with full details (title, description, ingredients, instructions, images, timing, difficulty)

**Independent Test**: Create a recipe with title, ingredients, and instructions, then view it to confirm all entered information displays correctly.

### Models for US1

- [x] T027 Create Recipe table migration in migrations/003_recipes.sql
- [x] T028 Create Ingredient table migration in migrations/004_ingredients.sql
- [x] T029 Create InstructionStep table migration in migrations/005_instruction_steps.sql
- [x] T030 Create RecipeImage table migration in migrations/006_recipe_images.sql
- [x] T031 [P] [US1] Implement Recipe model in src/models/recipe.rs
- [x] T032 [P] [US1] Implement Ingredient model in src/models/ingredient.rs
- [x] T033 [P] [US1] Implement InstructionStep model in src/models/instruction_step.rs
- [x] T034 [P] [US1] Implement RecipeImage model in src/models/recipe_image.rs

### Services for US1

- [x] T035 [US1] Implement RecipeService in src/services/recipe_service.rs with create, get_by_id, update, delete, list_by_author methods
- [x] T036 [US1] Implement ingredient validation (max 50) in RecipeService
- [x] T037 [US1] Implement instruction validation (max 30 steps) in RecipeService
- [x] T038 [US1] Implement Schema.org JSON-LD serialization in src/lib/schema_org.rs

### Image Upload for US1

- [x] T039 [US1] Setup S3 client in src/lib/storage.rs with configurable backend
- [x] T040 [US1] Implement image upload service in src/services/image_service.rs (resize, validate, upload)
- [x] T041 [US1] Implement image limit validation (max 10 per recipe) in image_service.rs

### API Endpoints for US1

- [x] T042 [US1] Implement POST /api/v1/recipes in src/api/recipes.rs (create recipe)
- [x] T043 [US1] Implement GET /api/v1/recipes/{id} in src/api/recipes.rs (get recipe with JSON and JSON-LD; support Accept header content negotiation per Constitution III)
- [x] T044 [US1] Implement PUT /api/v1/recipes/{id} in src/api/recipes.rs (update recipe)
- [x] T045 [US1] Implement DELETE /api/v1/recipes/{id} in src/api/recipes.rs (delete recipe)
- [x] T046 [US1] Implement GET /api/v1/recipes in src/api/recipes.rs (list recipes with pagination)
- [x] T047 [US1] Implement POST /api/v1/recipes/{id}/images in src/api/recipes.rs (upload image)

### HTML Handlers for US1

- [x] T048 [P] [US1] Create recipe card component template in templates/components/recipe_card.html
- [x] T049 [P] [US1] Create ingredient list component template in templates/components/ingredient_list.html
- [x] T050 [P] [US1] Create instruction steps component template in templates/components/instruction_steps.html
- [x] T051 [US1] Create recipe create/edit form page in templates/recipes/form.html with HTMX
- [x] T052 [US1] Create recipe view page in templates/recipes/view.html
- [x] T053 [US1] Create recipe list page in templates/recipes/list.html
- [x] T054 [US1] Implement HTML handlers in src/handlers/recipes.rs (create, view, edit, list pages)
- [x] T055 [US1] Create user profile page showing their recipes in templates/users/profile.html
- [x] T056 [US1] Implement user profile handler in src/handlers/users.rs

### Metric/Imperial Conversion for US1

- [x] T057 [US1] Implement unit conversion utilities in src/lib/units.rs (metric ↔ imperial)
- [x] T058 [US1] Apply user's measurement preference when rendering ingredients in templates

### Seeds & Fixtures for MVP Testing

- [x] T059 Create src/lib/seed.rs with seed runner, environment detection (dev/test), and CLI flag `--seed`
- [x] T060 Create seed data: 3 test users (alice/metric, bob/imperial, chef_marie/metric) in src/lib/seeds/users.rs
- [x] T061 Create seed data: 5 sample recipes with varying complexity (simple 3-ingredient to complex 15-ingredient) in src/lib/seeds/recipes.rs
- [x] T062 Create seed data: 1 "stress test" recipe at validation limits (50 ingredients, 30 steps, 10 images) in src/lib/seeds/recipes.rs
- [x] T063 Add seed command to Makefile (`make seed` runs with --seed flag, `make reset-db` drops + migrates + seeds)

**Checkpoint**: User Story 1 complete - users can create, view, edit, delete recipes with all details

---

## Phase 4: User Story 2 - Organize Recipes into Books (Priority: P2)

**Goal**: Users can create recipe books (collections), add/remove recipes, and organize their culinary content

**Independent Test**: Create a recipe book, add recipes to it, verify the book displays contained recipes correctly

**Dependency**: Requires Recipe entity from US1

### Models for US2

- [ ] T064 Create RecipeBook table migration in migrations/007_recipe_books.sql
- [ ] T065 Create BookRecipeEntry table migration in migrations/008_book_recipe_entries.sql
- [ ] T066 [P] [US2] Implement RecipeBook model in src/models/recipe_book.rs
- [ ] T067 [P] [US2] Implement BookRecipeEntry model in src/models/book_recipe_entry.rs

### Services for US2

- [ ] T068 [US2] Implement BookService in src/services/book_service.rs with create, get_by_id, update, delete, list_by_owner methods
- [ ] T069 [US2] Implement add_recipe, remove_recipe methods in BookService
- [ ] T070 [US2] Implement get_recipes_in_book method in BookService

### API Endpoints for US2

- [ ] T071 [US2] Implement POST /api/v1/books in src/api/books.rs (create book with multipart/form-data for optional cover image per FR-011)
- [ ] T072 [US2] Implement GET /api/v1/books/{id} in src/api/books.rs (get book with recipes)
- [ ] T073 [US2] Implement PUT /api/v1/books/{id} in src/api/books.rs (update book)
- [ ] T074 [US2] Implement DELETE /api/v1/books/{id} in src/api/books.rs (delete book)
- [ ] T075 [US2] Implement GET /api/v1/books in src/api/books.rs (list books with pagination)
- [ ] T076 [US2] Implement POST /api/v1/books/{id}/recipes in src/api/books.rs (add recipe to book)
- [ ] T077 [US2] Implement DELETE /api/v1/books/{id}/recipes/{recipeId} in src/api/books.rs (remove recipe)

### HTML Handlers for US2

- [ ] T078 [P] [US2] Create book card component template in templates/components/book_card.html
- [ ] T079 [US2] Create book create/edit form page in templates/books/form.html with HTMX
- [ ] T080 [US2] Create book view page in templates/books/view.html (with recipe list)
- [ ] T081 [US2] Create book list page in templates/books/list.html
- [ ] T082 [US2] Implement HTML handlers in src/handlers/books.rs
- [ ] T083 [US2] Add "Add to Book" button to recipe view page with HTMX dropdown

**Checkpoint**: User Story 2 complete - users can create books and organize recipes

---

## Phase 5: User Story 3 - Share Recipes and Books (Priority: P3)

**Goal**: Users can follow others, save recipes, share to activity feed, and see updates from followed users

**Independent Test**: Share a recipe, have another user view it, confirm they can see and interact with the shared content

**Dependencies**: Requires Recipe (US1) and RecipeBook (US2) entities

### Models for US3

- [ ] T084 Create Follow table migration in migrations/009_follows.sql
- [ ] T085 Create SavedRecipe table migration in migrations/010_saved_recipes.sql
- [ ] T086 Create Activity table migration in migrations/011_activities.sql
- [ ] T087 [P] [US3] Implement Follow model in src/models/follow.rs
- [ ] T088 [P] [US3] Implement SavedRecipe model in src/models/saved_recipe.rs
- [ ] T089 [P] [US3] Implement Activity model in src/models/activity.rs (ensure created_at uses TIMESTAMPTZ with ISO 8601 serialization per Constitution III)

### Services for US3

- [ ] T090 [US3] Implement FollowService in src/services/follow_service.rs with follow, unfollow, get_followers, get_following methods
- [ ] T091 [US3] Implement SavedRecipeService in src/services/saved_recipe_service.rs with save, unsave, get_saved methods
- [ ] T092 [US3] Implement ActivityService in src/services/activity_service.rs with create_activity, get_feed methods
- [ ] T093 [US3] Create activity when recipe/book is created (integrate with RecipeService, BookService)
- [ ] T094 [US3] Implement share action creating Announce activity in ActivityService

### API Endpoints for US3

- [ ] T095 [US3] Implement POST /api/v1/users/{id}/follow in src/api/social.rs
- [ ] T096 [US3] Implement DELETE /api/v1/users/{id}/follow in src/api/social.rs
- [ ] T097 [US3] Implement POST /api/v1/recipes/{id}/save in src/api/social.rs
- [ ] T098 [US3] Implement DELETE /api/v1/recipes/{id}/save in src/api/social.rs
- [ ] T099 [US3] Implement GET /api/v1/users/{id}/saved in src/api/social.rs (list saved recipes)
- [ ] T100 [US3] Implement POST /api/v1/recipes/{id}/share in src/api/social.rs
- [ ] T101 [US3] Implement GET /api/v1/feed in src/api/social.rs (activity feed with pagination)

### HTML Handlers for US3

- [ ] T102 [P] [US3] Create activity card component template in templates/components/activity_card.html
- [ ] T103 [US3] Create activity feed page in templates/feed/index.html
- [ ] T104 [US3] Implement feed handler in src/handlers/feed.rs
- [ ] T105 [US3] Add follow/unfollow button to user profile page with HTMX
- [ ] T106 [US3] Add save/unsave button to recipe view page with HTMX
- [ ] T107 [US3] Add share button to recipe view page with HTMX
- [ ] T108 [US3] Create saved recipes page in templates/users/saved.html
- [ ] T109 [US3] Update user profile to show follower/following counts

**Checkpoint**: User Story 3 complete - social features working (follow, save, share, feed)

---

## Phase 6: User Story 4 - Control Recipe Visibility (Priority: P4)

**Goal**: Users can set recipes and books to public or private, private content is hidden from other users

**Independent Test**: Set a recipe to private, confirm it's not visible to other users while remaining visible to the author

**Dependencies**: Visibility field already in Recipe/RecipeBook models from US1/US2

### Implementation for US4

- [ ] T110 [US4] Add visibility filter to RecipeService.list_public method in src/services/recipe_service.rs
- [ ] T111 [US4] Add visibility filter to BookService.list_public method in src/services/book_service.rs
- [ ] T112 [US4] Add authorization check in RecipeService.get_by_id (return 404 for private + not owner)
- [ ] T113 [US4] Add authorization check in BookService.get_by_id (return 404 for private + not owner)
- [ ] T114 [US4] Add visibility toggle to recipe form in templates/recipes/form.html
- [ ] T115 [US4] Add visibility toggle to book form in templates/books/form.html
- [ ] T116 [US4] Add visibility badge to recipe_card and book_card components
- [ ] T117 [US4] Filter private recipes from book view for non-owners

**Checkpoint**: User Story 4 complete - privacy controls working

---

## Phase 7: ActivityPub Federation

**Goal**: Recipes and books are representable as ActivityPub objects for federation (per constitution)

**Dependencies**: All entities from US1-US3

- [ ] T118 Implement ActivityPub Actor for User in src/lib/activitypub/actor.rs
- [ ] T119 Implement ActivityPub Object for Recipe in src/lib/activitypub/recipe.rs
- [ ] T120 Implement ActivityPub Collection for RecipeBook in src/lib/activitypub/book.rs
- [ ] T121 Implement WebFinger endpoint in src/api/webfinger.rs
- [ ] T122 Implement ActivityPub inbox handler in src/api/activitypub.rs
- [ ] T123 Implement ActivityPub outbox handler in src/api/activitypub.rs
- [ ] T124 Generate ap_id for new entities in services
- [ ] T125 Implement HTTP signature verification for incoming activities
- [ ] T126 Implement HTTP signature signing for outgoing activities
- [ ] T127 Create background job for federation delivery in src/jobs/federation.rs

**Checkpoint**: Federation infrastructure ready

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T128 [P] Add rate limiting middleware with tower-governor in src/api/middleware/rate_limit.rs
- [ ] T129 [P] Add request logging with tracing in src/main.rs
- [ ] T130 [P] Create health check endpoint GET /health in src/api/health.rs
- [ ] T131 [P] Add OpenAPI documentation generation (utoipa or similar)
- [ ] T132 Implement input validation with validator crate across all endpoints
- [ ] T133 Add proper error responses matching OpenAPI spec
- [ ] T134 Create database indexes per data-model.md specifications

### Accessibility (Constitution v1.2.0 - State of the Art)

- [ ] T135 Validate skip links work correctly on all pages (keyboard focus, visibility on focus)
- [ ] T136 Validate ARIA landmarks (main, nav, banner, contentinfo) present and correct on all page templates
- [ ] T137 Ensure all form inputs have associated labels with `for` attribute; link error messages with `aria-describedby`
- [ ] T138 Add visible focus indicators (3:1 contrast) to all interactive elements in Tailwind config
- [ ] T139 Implement high-contrast mode toggle in user preferences and CSS custom properties
- [ ] T140 Add alt text prompt/requirement to image upload forms; mark decorative images with `alt=""`
- [ ] T141 Ensure color is not sole information carrier (add icons/text to status indicators)
- [ ] T142 Run axe-core automated audit on all pages; fix all critical/serious issues
- [ ] T143 Run Lighthouse accessibility audit; achieve score ≥95
- [ ] T144 Manual keyboard navigation test: verify all journeys completable without mouse
- [ ] T145 Manual screen reader test (NVDA or VoiceOver): verify all content accessible and properly announced
- [ ] T146 Verify AAA color contrast (7:1 normal, 4.5:1 large) across all color combinations
- [ ] T147 Verify core journeys complete in ≤3 clicks: browse recipes, search, view recipe, follow user (Constitution IV)
- [ ] T148 Validate responsive layout on 320px viewport; ensure all features usable on mobile-first breakpoints
- [ ] T149 Implement prefers-reduced-motion CSS support; disable animations/transitions when user prefers reduced motion

### Final Polish

- [ ] T150 Optimize page load performance (<3s on 3G target)
- [ ] T151 Run quickstart.md validation scenarios
- [ ] T152 Create CONTRIBUTING.md with development setup instructions

### Constitution Compliance & Quality

- [ ] T153 [P] Implement RSS/Atom feeds for public recipes and user profiles in src/api/feeds.rs (Constitution III: RSS/Atom)
- [ ] T154 [P] Implement oEmbed endpoint for recipe embedding in src/api/oembed.rs (Constitution III: Embedding)
- [ ] T155 [P] Document encryption-at-rest requirements for PostgreSQL and S3 in DEPLOYMENT.md (Constitution II: Security)
- [ ] T156 [P] Setup background job queue infrastructure with tokio tasks or sqlx-based queue in src/jobs/mod.rs (Constitution V: Background Jobs)
- [ ] T157 Run load testing with k6 to verify SC-007 (1000 concurrent users) and document results
- [ ] T158 Add UX metrics instrumentation for SC-001/002/003 (recipe creation time, book creation time, first-publish success rate)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational - MVP, can start first
- **User Story 2 (Phase 4)**: Depends on Foundational + Recipe entity from US1
- **User Story 3 (Phase 5)**: Depends on Foundational + entities from US1 and US2
- **User Story 4 (Phase 6)**: Depends on Foundational - can run parallel with US2/US3
- **Federation (Phase 7)**: Depends on all user story entities being complete
- **Polish (Phase 8)**: Can start partially after US1, complete after all stories

### User Story Dependencies

```
Foundational (Phase 2)
        │
        ├──────────────────────┬──────────────────────┐
        ▼                      ▼                      ▼
   US1 (P1) ◄────────────  US4 (P4)               [parallel]
   Recipe CRUD              Visibility
        │                      │
        ▼                      │
   US2 (P2) ◄──────────────────┘
   Recipe Books
        │
        ▼
   US3 (P3)
   Social/Sharing
        │
        ▼
   Federation (Phase 7)
```

### Parallel Opportunities Per Phase

**Phase 1 (Setup)**: T003, T004, T005, T006, T007, T008, T011, T012 can all run in parallel

**Phase 2 (Foundational)**: T017, T018, T019, T021 can run in parallel after DB setup

**Phase 3 (US1)**:
- T031, T032, T033, T034 (all models) in parallel
- T048, T049, T050 (components) in parallel

**Phase 4 (US2)**: T066, T067 (models) in parallel, T078 (component) parallel with models

**Phase 5 (US3)**: T087, T088, T089 (models) in parallel, T102 (component) parallel with models

**Phase 8 (Polish)**: T128, T129, T130, T131, T153, T154, T155, T156 can all run in parallel

---

## Parallel Example: User Story 1 Models

```bash
# Launch all US1 models together (after migrations complete):
Task: "Implement Recipe model in src/models/recipe.rs"
Task: "Implement Ingredient model in src/models/ingredient.rs"
Task: "Implement InstructionStep model in src/models/instruction_step.rs"
Task: "Implement RecipeImage model in src/models/recipe_image.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test recipe creation independently
5. Deploy/demo if ready - users can create and view recipes

### Incremental Delivery

1. **Foundation** → Setup + Foundational complete
2. **MVP** → Add US1 (Recipe CRUD) → Deploy
3. **Organization** → Add US2 (Books) → Deploy
4. **Social** → Add US3 (Follow/Share/Feed) → Deploy
5. **Privacy** → Add US4 (Visibility controls) → Deploy
6. **Federation** → Add Phase 7 → Deploy

### Suggested MVP Scope

**Minimum viable product**: Phase 1 + Phase 2 + Phase 3 (User Story 1)
- Users can create, view, edit, delete recipes
- Basic profile showing user's recipes
- Schema.org JSON-LD output
- Development seeds with test users and sample recipes
- Accessibility foundations (skip links, ARIA landmarks in base layout)
- AGPL-3.0 license and cargo-audit CI in place
- 63 tasks (T001-T063)

**Total project tasks**: 158 (T001-T158)

---

## Notes

- [P] tasks = different files, no dependencies within that phase
- [Story] label maps task to specific user story for traceability
- Unit tests go inline in each .rs file with `#[cfg(test)]`
- Migrations should be run in order (numbered)
- Constitution compliance: all phases include federation, accessibility, security considerations

## Commit Convention

Each task MUST result in exactly one commit following [Conventional Commits](https://www.conventionalcommits.org/) (see Constitution § Commit Conventions).

**Format**: `type(scope): description` (max 72 chars, lowercase, imperative)

**Task → Commit Type Mapping**:

| Task Pattern | Commit Type | Example |
|--------------|-------------|---------|
| Create migration | `chore(db)` | `chore(db): create recipe table migration` |
| Implement model | `feat(model)` | `feat(model): implement recipe with sqlx derives` |
| Implement service | `feat(service)` | `feat(service): add recipe crud operations` |
| Implement endpoint | `feat(api)` | `feat(api): add POST /recipes endpoint` |
| Create template | `feat(ui)` | `feat(ui): create recipe card component` |
| Implement handler | `feat(handler)` | `feat(handler): add recipe list page` |
| Add validation | `feat(validation)` | `feat(validation): limit ingredients to 50` |
| Create seed data | `chore(seed)` | `chore(seed): add sample recipes` |
| Setup tooling | `chore(tooling)` | `chore(tooling): configure tailwind cli` |
| Add middleware | `feat(middleware)` | `feat(middleware): add rate limiting` |
| Add index | `perf(db)` | `perf(db): add recipe visibility index` |
| Accessibility | `feat(a11y)` | `feat(a11y): add skip links to base layout` |
| Accessibility audit | `test(a11y)` | `test(a11y): run axe-core audit and fix issues` |
| Fix bug | `fix(scope)` | `fix(auth): prevent self-follow` |
