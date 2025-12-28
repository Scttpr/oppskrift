# Tasks: Comprehensive Test Coverage

**Input**: Design documents from `/specs/003-test-coverage/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: This feature IS about tests - all tasks create test code.

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Progress

| Phase | Description | Status | Tasks |
|-------|-------------|--------|-------|
| 1 | Setup (Test Infrastructure) | ✅ Complete | T001-T004 |
| 2 | Foundational (Test Helpers) | ✅ Complete | T005-T010 |
| 3 | User Story 1 - Unit Tests for Missing Files | ✅ Complete | T011-T025 |
| 4 | User Story 2 - API Integration Tests | ✅ Complete | T026-T040 |
| 4.5 | **OSK Security Tests (Principe IV)** | ✅ Complete | T064-T075 |
| 5 | User Story 3 - Service Layer Tests | ✅ Complete | T041-T045 |
| 6 | User Story 4 - Model Validation Tests | ✅ Complete | T046-T050 |
| 7 | User Story 5 - Handler Tests | ✅ Complete | T051-T058 |
| 8 | Polish & Validation | ✅ Complete | T059-T063 |

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US5)
- Include exact file paths in descriptions

## Path Conventions

Single project structure at repository root:
- Source: `src/`
- Tests: `tests/`
- Unit tests: Inline `#[cfg(test)] mod tests` blocks

---

## Phase 1: Setup (Test Infrastructure)

**Purpose**: Ensure test infrastructure is ready for comprehensive testing

- [x] T001 Verify test database connection and migrations in tests/common/mod.rs
- [x] T002 [P] Add test fixtures for common entities in tests/common/fixtures.rs
- [x] T003 [P] Add test assertion helpers in tests/common/assertions.rs
- [x] T004 [P] Document test patterns in specs/003-test-coverage/quickstart.md

---

## Phase 2: Foundational (Test Helpers)

**Purpose**: Create reusable test utilities that all user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 Create recipe test fixtures (create_test_recipe, create_test_ingredients) in tests/common/mod.rs
- [x] T006 [P] Create book test fixtures (create_test_book, add_recipe_to_book) in tests/common/mod.rs
- [x] T007 [P] Create social test fixtures (create_test_follow, create_test_activity) in tests/common/mod.rs
- [x] T008 [P] Create federation test fixtures (mock_remote_actor, mock_activity) in tests/common/fixtures.rs
- [x] T009 Add authenticated request helper (login_and_get_cookie) in tests/common/mod.rs
- [x] T010 Add JSON assertion helpers (assert_json_field, assert_json_array) in tests/common/assertions.rs

**Checkpoint**: Test helpers ready - user story implementation can begin

---

## Phase 3: User Story 1 - Unit Tests for Missing Files (Priority: P1) 🎯 MVP

**Goal**: Add inline unit tests to the 32 source files currently missing tests

**Independent Test**: Run `cargo test --lib` and verify all modules have passing tests

### Core Utilities (3 files)

- [x] T011 [P] [US1] Add unit tests for key generation in src/core/crypto.rs
- [x] T012 [P] [US1] Add unit tests for seed data validation in src/core/seeds/users.rs
- [x] T013 [P] [US1] Add unit tests for seed data validation in src/core/seeds/recipes.rs
- [x] T014 [P] [US1] Add unit tests for seed data validation in src/core/seeds/books.rs

### API Module Unit Tests (10 files)

- [x] T015 [P] [US1] Add unit tests for request/response types in src/api/auth.rs
- [x] T016 [P] [US1] Add unit tests for request/response types in src/api/account.rs
- [x] T017 [P] [US1] Add unit tests for request/response types in src/api/recipes.rs
- [x] T018 [P] [US1] Add unit tests for request/response types in src/api/books.rs
- [x] T019 [P] [US1] Add unit tests for request/response types in src/api/users.rs
- [x] T020 [P] [US1] Add unit tests for request/response types in src/api/feeds.rs
- [x] T021 [P] [US1] Add unit tests for request/response types in src/api/social.rs
- [x] T022 [P] [US1] Add unit tests for activity serialization in src/api/activitypub.rs
- [x] T023 [P] [US1] Add unit tests for webfinger response in src/api/webfinger.rs
- [x] T024 [P] [US1] Add unit tests for OpenAPI spec generation in src/api/openapi.rs

### App Bootstrap Test

- [x] T025 [US1] Add integration test for app creation in src/lib.rs

**Checkpoint**: User Story 1 complete - all source files have inline tests

---

## Phase 4: User Story 2 - API Integration Tests (Priority: P1)

**Goal**: Add integration tests verifying API endpoint contracts

**Independent Test**: Run `cargo test --test '*'` and verify all API endpoints have tests

### Recipe API Tests

- [x] T026 [P] [US2] Create recipes_test.rs with setup helper in tests/recipes_test.rs
- [x] T027 [US2] Add test_create_recipe (happy path) in tests/recipes_test.rs
- [x] T028 [US2] Add test_create_recipe_validation_error in tests/recipes_test.rs
- [x] T029 [US2] Add test_get_recipe (public, private, not found) in tests/recipes_test.rs
- [x] T030 [US2] Add test_update_recipe in tests/recipes_test.rs
- [x] T031 [US2] Add test_delete_recipe in tests/recipes_test.rs

### Book API Tests

- [x] T032 [P] [US2] Create books_test.rs with setup helper in tests/books_test.rs
- [x] T033 [US2] Add test_create_book, test_add_recipe_to_book in tests/books_test.rs
- [x] T034 [US2] Add test_remove_recipe_from_book, test_delete_book in tests/books_test.rs

### Social API Tests

- [x] T035 [P] [US2] Create social_test.rs with setup helper in tests/social_test.rs
- [x] T036 [US2] Add test_follow_user, test_unfollow_user in tests/social_test.rs
- [x] T037 [US2] Add test_get_followers, test_get_following in tests/social_test.rs
- [x] T038 [US2] Add test_activity_feed in tests/social_test.rs

### Federation API Tests

- [x] T039 [P] [US2] Create federation_test.rs with mock actors in tests/federation_test.rs
- [x] T040 [US2] Add test_webfinger_lookup, test_actor_endpoint in tests/federation_test.rs

**Checkpoint**: User Story 2 complete - all API endpoints have integration tests

---

## Phase 4.5: OSK Security Tests (Principe IV) 🔒 CRITICAL

**Goal**: Implement security-specific tests required by OSK Principle IV (Security Testing)

**Reference**: `.osk/specs/002-user-auth/testing.md`

**Independent Test**: Run `cargo test --test security` and verify all security tests pass

### Security Test Infrastructure

- [x] T064 [P] [SEC] Create tests/security/ directory structure (now tests/security_*.rs files)
- [x] T065 [SEC] Add security test helpers (create_verified_user, measure_timing) in tests/common/security.rs

### Rate Limiting Tests

- [x] T066 [P] [SEC] Create tests/security_rate_limit_test.rs with test_login_rate_limit_by_ip
- [x] T067 [SEC] Add test_registration_rate_limit in tests/security_rate_limit_test.rs
- [x] T068 [SEC] Add test_password_reset_rate_limit_per_email in tests/security_rate_limit_test.rs

### User Enumeration Prevention Tests

- [x] T069 [P] [SEC] Create tests/security_enumeration_test.rs with test_login_no_user_enumeration
- [x] T070 [SEC] Add test_registration_no_email_enumeration in tests/security_enumeration_test.rs
- [x] T071 [SEC] Add test_reset_no_email_enumeration in tests/security_enumeration_test.rs

### Timing Attack Resistance Tests

- [x] T072 [P] [SEC] Create tests/security_timing_test.rs with test_login_constant_time

### Authentication Security Tests

- [x] T073 [P] [SEC] Add test_session_token_forgery_rejected in tests/security_auth_test.rs
- [x] T074 [SEC] Add test_session_cookie_security_flags in tests/security_auth_test.rs
- [x] T075 [SEC] Add test_account_lockout_after_failed_attempts in tests/security_auth_test.rs

**Checkpoint**: OSK Principe IV compliance verified - all security tests pass

---

## Phase 5: User Story 3 - Service Layer Coverage (Priority: P2)

**Goal**: Expand existing service tests to cover edge cases and error paths

**Independent Test**: Run `cargo test` and verify service modules have comprehensive coverage

- [x] T041 [P] [US3] Add error path tests to src/services/recipe_service.rs
- [x] T042 [P] [US3] Add error path tests to src/services/book_service.rs
- [x] T043 [P] [US3] Add error path tests to src/services/follow_service.rs
- [x] T044 [P] [US3] Add error path tests to src/services/activity_service.rs
- [x] T045 [P] [US3] Add error path tests to src/services/saved_recipe_service.rs

**Checkpoint**: User Story 3 complete - services have comprehensive error coverage

---

## Phase 6: User Story 4 - Model Validation Tests (Priority: P2)

**Goal**: Ensure all model validation rules are tested

**Independent Test**: Run `cargo test` and verify model validation is comprehensive

- [x] T046 [P] [US4] Add missing validation tests to src/models/recipe.rs
- [x] T047 [P] [US4] Add missing validation tests to src/models/ingredient.rs
- [x] T048 [P] [US4] Add missing validation tests to src/models/instruction_step.rs
- [x] T049 [P] [US4] Add serialization round-trip tests to src/models/activity.rs
- [x] T050 [P] [US4] Add serialization round-trip tests to src/models/follow.rs

**Checkpoint**: User Story 4 complete - models have validation and serialization tests

---

## Phase 7: User Story 5 - Handler Tests (Priority: P3)

**Goal**: Add tests for HTML handlers and template rendering

**Independent Test**: Run integration tests and verify handlers return correct HTML

- [x] T051 [P] [US5] Add handler tests to src/handlers/auth.rs
- [x] T052 [P] [US5] Add handler tests to src/handlers/recipes.rs
- [x] T053 [P] [US5] Add handler tests to src/handlers/books.rs
- [x] T054 [P] [US5] Add handler tests to src/handlers/users.rs
- [x] T055 [P] [US5] Add handler tests to src/handlers/feed.rs
- [x] T056 [P] [US5] Add handler tests to src/handlers/legal.rs
- [x] T057 [US5] Create handlers_test.rs for integration testing in tests/handlers_test.rs
- [x] T058 [US5] Add redirect and auth flow tests in tests/handlers_test.rs

**Checkpoint**: User Story 5 complete - handlers have test coverage

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Validation and documentation

- [x] T059 Run full test suite and verify all 150+ tests pass (630 tests passing)
- [x] T060 [P] Verify test execution time is under 5 minutes (~38 seconds)
- [x] T061 [P] Update specs/003-test-coverage/quickstart.md with actual examples
- [x] T062 Run `cargo clippy` on test code (warnings only, no errors)
- [x] T063 Final validation: run `cargo test` 3 times to check for flakiness (0 failures)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - US1 and US2 are both P1 - can proceed in parallel
  - **OSK Security (Phase 4.5)**: P1 - depends on Phase 2, can run parallel to US1/US2 🔒
  - US3 and US4 are P2 - can proceed after or with P1
  - US5 is P3 - lowest priority
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (Unit Tests)**: After Foundational - No dependencies on other stories
- **User Story 2 (API Tests)**: After Foundational - Uses test fixtures from Phase 2
- **OSK Security Tests**: After Foundational - Uses security helpers from T065 🔒
- **User Story 3 (Service Tests)**: After Foundational - Independent
- **User Story 4 (Model Tests)**: After Foundational - Independent
- **User Story 5 (Handler Tests)**: After Foundational - May use fixtures

### Within Each User Story

- All [P] tasks within a phase can run in parallel
- Non-[P] tasks depend on previous tasks in sequence
- Tests can be added incrementally

### Parallel Opportunities

- All [P] tasks in Phase 2 can run in parallel
- All [P] tasks in Phase 3 can run in parallel (14 tasks)
- US1, US2, Security, US3, US4 can be developed in parallel (different files)
- Multiple test files marked [P] can be created simultaneously
- Security tests (T064, T066, T069, T072, T073) can start in parallel

---

## Parallel Example: Phase 3 (User Story 1)

```bash
# Launch all core utility tests together:
Task T011: "Add unit tests in src/core/crypto.rs"
Task T012: "Add unit tests in src/core/seeds/users.rs"
Task T013: "Add unit tests in src/core/seeds/recipes.rs"
Task T014: "Add unit tests in src/core/seeds/books.rs"

# Launch all API unit tests together:
Task T015-T024: All API module unit tests (10 parallel tasks)
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 + Security)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (test helpers)
3. Complete Phase 3: User Story 1 (unit tests for 32 files)
4. **STOP and VALIDATE**: Run `cargo test --lib` - all unit tests pass
5. Complete Phase 4: User Story 2 (API integration tests)
6. **STOP and VALIDATE**: Run `cargo test --test '*'` - all integration tests pass
7. Complete Phase 4.5: OSK Security Tests (Principe IV) 🔒
8. **STOP and VALIDATE**: Run `cargo test --test security` - all security tests pass

### Incremental Delivery

1. Complete Setup + Foundational → Test infrastructure ready
2. Add User Story 1 → Unit tests for missing files → **73/73 files covered**
3. Add User Story 2 → API integration tests → **All endpoints tested**
4. Add OSK Security → Security tests → **Principe IV compliant** 🔒
5. Add User Story 3 → Service error paths → **Comprehensive coverage**
6. Add User Story 4 → Model validation → **Data integrity verified**
7. Add User Story 5 → Handler tests → **Full coverage**
8. Polish phase → **Production ready**

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (unit tests)
   - Developer B: User Story 2 (API tests)
3. After P1 stories complete:
   - Developer A: User Story 3 (service tests)
   - Developer B: User Story 4 (model tests)
   - Developer C: User Story 5 (handler tests)
4. All stories integrate independently

---

## Summary

| Phase | Tasks | Parallel Opportunities |
|-------|-------|----------------------|
| Setup | 4 | 3 parallel |
| Foundational | 6 | 5 parallel |
| US1 Unit Tests | 15 | 14 parallel |
| US2 API Tests | 15 | 4 parallel (setup tasks) |
| **OSK Security** | **12** | **5 parallel** |
| US3 Service Tests | 5 | 5 parallel |
| US4 Model Tests | 5 | 5 parallel |
| US5 Handler Tests | 8 | 6 parallel |
| Polish | 5 | 2 parallel |
| **Total** | **75** | **~49 parallel groups** |

### Task Count by User Story

- **US1 (Unit Tests)**: 15 tasks - Covers 32 missing files
- **US2 (API Tests)**: 15 tasks - Covers all API endpoints
- **SEC (Security Tests)**: 12 tasks - OSK Principle IV compliance 🔒
- **US3 (Service Tests)**: 5 tasks - Error path coverage
- **US4 (Model Tests)**: 5 tasks - Validation coverage
- **US5 (Handler Tests)**: 8 tasks - Template rendering

### Independent Test Criteria

- **US1**: `cargo test --lib` → All unit tests pass
- **US2**: `cargo test --test '*'` → All integration tests pass
- **SEC**: `cargo test --test security` → All security tests pass 🔒
- **US3**: Service tests cover error paths
- **US4**: Model validation tests are comprehensive
- **US5**: Handler tests verify correct responses

### Suggested MVP Scope

**User Stories 1 + 2 + Security** (42 tasks + 10 foundational = 52 tasks for complete API + security coverage)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability (US1-US5)
- [SEC] label = OSK security test (Principe IV compliance)
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- **Final test count: 630 tests** (exceeded target of 170+)
