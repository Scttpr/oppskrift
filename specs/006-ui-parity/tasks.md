# Tasks: UI Parity

**Input**: Design documents from `/specs/006-ui-parity/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Not explicitly requested - test tasks omitted. Add integration tests in Polish phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `templates/`, `tests/` at repository root
- Paths follow existing Oppskrift structure

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Database migration for contribution status field

- [x] T001 Create migration file for book_contributions status column in migrations/YYYYMMDDHHMMSS_add_contribution_status.sql
- [x] T002 Run migration and regenerate SQLx query cache with `cargo sqlx prepare`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extend existing models and services with view types needed across multiple stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T003 [P] Add SessionItemView struct to src/models/session.rs for template display
- [x] T004 [P] Add SecurityEventView struct to src/api/account.rs for template display
- [x] T005 [P] Add UserCardView struct to src/models/user.rs for follower/following display
- [x] T006 [P] Update BookContribution model in src/models/book_contribution.rs with status and rejection_reason fields
- [x] T007 Update BookContributionService in src/services/book_contribution_service.rs with accept/reject methods
- [x] T008 Add Privacy link to settings navigation in templates/settings/_nav.html

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - View and Manage Sessions (Priority: P1) 🎯 MVP

**Goal**: Users can view all active sessions and revoke individual sessions

**Independent Test**: Login from multiple browsers, view sessions list, revoke one session, verify it's terminated

### Implementation for User Story 1

- [x] T009 [US1] Create SessionsPageTemplate struct in src/handlers/settings.rs
- [x] T010 [US1] Implement sessions_page handler to list all sessions in src/handlers/settings.rs
- [x] T011 [US1] Implement revoke_single_session handler (HTMX) in src/handlers/settings.rs
- [x] T012 [US1] Extend templates/settings/sessions.html with full session table, per-session revoke buttons, and verify "Revoke All Other Sessions" works (FR-004)
- [x] T013 [US1] Add route registration for /security/sessions/{session_id}/revoke in src/handlers/settings.rs

**Checkpoint**: User Story 1 complete - session management functional and testable

---

## Phase 4: User Story 2 - View Security Event History (Priority: P1)

**Goal**: Users can view paginated security event log

**Independent Test**: Perform login, change password, then view security events page to verify events appear with pagination

### Implementation for User Story 2

- [x] T014 [US2] Create SecurityEventsPageTemplate struct in src/handlers/settings.rs
- [x] T015 [US2] Implement security_events_page handler with pagination in src/handlers/settings.rs
- [x] T016 [US2] Create templates/settings/security_events.html with event list and pagination controls
- [x] T017 [US2] Add route registration for /security/events in src/handlers/settings.rs
- [x] T018 [US2] Add event type icons and human-readable labels mapping in template

**Checkpoint**: User Story 2 complete - security events viewer functional and testable

---

## Phase 5: User Story 3 - Toggle Federation Settings (Priority: P2)

**Goal**: Users can enable/disable ActivityPub federation from settings

**Independent Test**: Toggle federation off, verify WebFinger returns 404, toggle on, verify discoverable

### Implementation for User Story 3

- [x] T019 [P] [US3] Create PrivacyPageTemplate struct in src/handlers/settings.rs
- [x] T020 [US3] Implement privacy_page handler in src/handlers/settings.rs
- [x] T021 [US3] Implement toggle_federation handler (HTMX) in src/handlers/settings.rs
- [x] T022 [US3] Create templates/settings/privacy.html with federation toggle and explanation text
- [x] T023 [US3] Add route registrations for /privacy and /privacy/federation in src/handlers/settings.rs

**Checkpoint**: User Story 3 complete - federation toggle functional and testable

---

## Phase 6: User Story 4 - Export Personal Data (Priority: P2)

**Goal**: Users can download their personal data in JSON format

**Independent Test**: Click export button, verify JSON download contains profile, recipes, books

### Implementation for User Story 4

- [x] T024 [US4] Implement export_data handler in src/handlers/settings.rs
- [x] T025 [US4] Add export button and logic to templates/settings/privacy.html
- [x] T026 [US4] Implement sync export (<=50 recipes) with JSON download response
- [x] T027 [US4] Add route registration for /privacy/export in src/handlers/settings.rs
- [x] T028 [US4] Add rate limiting check (1 export per hour) in export handler

**Checkpoint**: User Story 4 complete - data export functional and testable

---

## Phase 7: User Story 5 - Manage Book Contributions (Priority: P2)

**Goal**: Book owners can view and accept/reject contributions; contributors see status

**Independent Test**: Contribute recipe to book (via API), owner views and accepts/rejects, contributor sees status

### Implementation for User Story 5

- [x] T029 [P] [US5] Create ContributionListView and ContributionItemView structs in src/handlers/books.rs
- [x] T030 [US5] Extend book_view handler to include contributions data in src/handlers/books.rs
- [x] T031 [US5] Implement accept_contribution handler (HTMX) in src/handlers/books.rs
- [x] T032 [US5] Implement reject_contribution handler (HTMX) with reason in src/handlers/books.rs
- [x] T033 [US5] Extend templates/books/view.html with contributions section
- [x] T034 [US5] Add contribution row partial for HTMX swap in templates/books/view.html
- [x] T035 [US5] Add route registrations for contribution accept/reject in src/handlers/books.rs

**Checkpoint**: User Story 5 complete - contribution management functional and testable

---

## Phase 8: User Story 6 - View Followers and Following Lists (Priority: P3)

**Goal**: Users can view follower/following lists with follow/unfollow actions

**Independent Test**: Follow users, view followers page, verify list accurate, unfollow from list

### Implementation for User Story 6

- [x] T036 [P] [US6] Create FollowersPageTemplate struct in src/handlers/users.rs
- [x] T037 [US6] Implement followers_page handler with pagination in src/handlers/users.rs
- [x] T038 [US6] Implement following_page handler with pagination in src/handlers/users.rs
- [x] T039 [P] [US6] Create templates/users/followers.html with user cards and follow buttons
- [x] T040 [P] [US6] Create templates/users/following.html with user cards and unfollow buttons
- [x] T041 [US6] Add route registrations for /users/{id}/followers and /users/{id}/following in src/handlers/users.rs
- [x] T042 [US6] Add follower/following count links in templates/users/profile.html

**Checkpoint**: User Story 6 complete - follower lists functional and testable

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Integration testing, cleanup, and validation

- [x] T043 [P] Create integration test file tests/ui_parity_test.rs
- [x] T044 [P] Add integration tests for sessions management endpoints
- [x] T045 [P] Add integration tests for security events page
- [x] T046 [P] Add integration tests for privacy settings and export
- [x] T047 [P] Add integration tests for contribution management
- [x] T048 [P] Add integration tests for follower/following pages
- [x] T049 Run cargo clippy and fix any warnings
- [x] T050 Run cargo fmt to format all code
- [x] T051 Validate all templates compile with cargo check
- [ ] T052 Run quickstart.md validation scenarios manually

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-8)**: All depend on Foundational phase completion
  - US1 and US2 are both P1 - implement in order or parallel
  - US3, US4, US5 are P2 - can proceed after P1 or in parallel
  - US6 is P3 - lowest priority
- **Polish (Phase 9)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Sessions - No dependencies on other stories
- **User Story 2 (P1)**: Security Events - No dependencies on other stories
- **User Story 3 (P2)**: Federation Toggle - No dependencies on other stories
- **User Story 4 (P2)**: Data Export - No dependencies on other stories
- **User Story 5 (P2)**: Contributions - Depends on T006/T007 from Foundational
- **User Story 6 (P3)**: Followers - No dependencies on other stories

### Within Each User Story

- View models before handlers
- Handlers before templates
- Templates before route registration
- Story complete before moving to next priority

### Parallel Opportunities

- T003, T004, T005, T006 can run in parallel (different files)
- T039, T040 can run in parallel (different template files)
- All integration tests (T044-T048) can run in parallel

---

## Parallel Example: Foundational Phase

```bash
# Launch all model/view updates together:
Task: "Add SessionItemView struct to src/models/session.rs"
Task: "Add SecurityEventView struct to src/api/account.rs"
Task: "Add UserCardView struct to src/models/user.rs"
Task: "Update BookContribution model in src/models/book_contribution.rs"
```

## Parallel Example: User Story 6

```bash
# Launch template creation together:
Task: "Create templates/users/followers.html"
Task: "Create templates/users/following.html"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2)

1. Complete Phase 1: Setup (migration)
2. Complete Phase 2: Foundational (view models, services)
3. Complete Phase 3: User Story 1 (Sessions)
4. Complete Phase 4: User Story 2 (Security Events)
5. **STOP and VALIDATE**: Both security-critical features work
6. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 (Sessions) → Test → Deploy (MVP!)
3. Add US2 (Security Events) → Test → Deploy
4. Add US3 (Federation) + US4 (Export) → Test → Deploy
5. Add US5 (Contributions) → Test → Deploy
6. Add US6 (Followers) → Test → Deploy
7. Polish phase → Final deploy

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- All handlers require AuthUser middleware (already applied to settings routes)
- Use HTMX for interactive elements (revoke, toggle, accept/reject)
- Follow existing template patterns from templates/settings/
