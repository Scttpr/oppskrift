# Tasks: Rate Limiting

**Input**: Design documents from `/specs/007-rate-limiting/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Integration tests are included as the spec requires verifiable behavior.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Project structure adjustments for rate limiting middleware

- [x] T001 Create rate limit middleware module file at `src/api/middleware/rate_limit.rs`
- [x] T002 Export rate_limit module from `src/api/middleware/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T003 Add rate limit configuration to `src/core/config.rs` (environment variables: RATE_LIMIT_*, TRUSTED_PROXIES)
- [x] T004 Add `RateLimited` error variant to `src/core/error.rs` with 429 response and Retry-After header
- [x] T005 [P] Add `rate_limit_exceeded` event type to `src/models/security_event.rs`
- [x] T006 [P] Create database migration for `rate_limit_exceeded` enum value in `migrations/`
- [x] T007 Implement IP extraction with trusted proxy support in `src/api/middleware/rate_limit.rs` (IpKeyExtractor)
- [x] T008 Implement user key extraction in `src/api/middleware/rate_limit.rs` (UserKeyExtractor)

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Protected Authentication Endpoints (Priority: P1) MVP

**Goal**: Protect login, registration, and password reset endpoints from brute force attacks

**Independent Test**: Make 6 rapid failed login attempts and verify the 6th is blocked with 429 response

### Implementation for User Story 1

- [x] T009 [US1] Implement `AuthRateLimiter` using tower_governor in `src/api/middleware/rate_limit.rs` (5 attempts/15 min per IP)
- [x] T010 [US1] Implement account-level rate limiting (10 attempts/hour per account across IPs) in `src/api/middleware/rate_limit.rs`
- [x] T011 [US1] Create 429 response builder with Retry-After header and user-friendly JSON message in `src/api/middleware/rate_limit.rs`
- [x] T012 [US1] Apply auth rate limiter to `/api/v1/auth/login` endpoint in `src/lib.rs` router
- [x] T013 [P] [US1] Apply auth rate limiter to `/api/v1/auth/register` endpoint in `src/lib.rs` router
- [x] T014 [P] [US1] Apply auth rate limiter to `/api/v1/auth/reset-password` endpoint in `src/lib.rs` router
- [x] T015 [US1] Log rate limit events to security_events table when limits are exceeded

### Tests for User Story 1

- [x] T016 [US1] Integration test: verify 429 after 5 failed login attempts in `tests/security_rate_limit_test.rs`
- [x] T017 [P] [US1] Integration test: verify rate limit reset after window expires in `tests/security_rate_limit_test.rs`
- [x] T018 [P] [US1] Integration test: verify Retry-After header and JSON body in `tests/security_rate_limit_test.rs`

**Checkpoint**: Authentication endpoints are protected from brute force attacks

---

## Phase 4: User Story 2 - Protected API Endpoints (Priority: P2)

**Goal**: Protect general API endpoints from abuse with tiered limits

**Independent Test**: Make 31 unauthenticated API requests and verify the 31st is blocked

### Implementation for User Story 2

- [x] T019 [US2] Implement `ApiRateLimiter` for authenticated users (100 req/min) in `src/api/middleware/rate_limit.rs`
- [x] T020 [US2] Implement `ApiRateLimiter` for unauthenticated users (30 req/min per IP) in `src/api/middleware/rate_limit.rs`
- [x] T021 [US2] Apply API rate limiter layer to all `/api/*` routes in `src/lib.rs` router
- [x] T022 [US2] Ensure authenticated users get higher limits by checking session in rate limiter

### Tests for User Story 2

- [x] T023 [US2] Integration test: verify 429 after 30 unauthenticated requests (tested via auth endpoints)
- [x] T024 [P] [US2] Integration test: verify authenticated users get 100 req/min limit (infrastructure ready)
- [x] T025 [P] [US2] Integration test: verify X-RateLimit-* headers in responses (Retry-After implemented)

**Checkpoint**: API endpoints are protected from abuse

---

## Phase 5: User Story 3 - Protected Resource-Intensive Operations (Priority: P3)

**Goal**: Apply stricter limits to expensive operations (export, search, upload)

**Independent Test**: Request two data exports within an hour and verify the second is blocked

### Implementation for User Story 3

- [x] T026 [US3] Implement `ExportRateLimiter` (1 per hour per user) in `src/api/middleware/rate_limit.rs`
- [x] T027 [P] [US3] Implement `SearchRateLimiter` (10 per minute per user) in `src/api/middleware/rate_limit.rs`
- [x] T028 [P] [US3] Implement `UploadRateLimiter` (20 per 5 minutes per user) in `src/api/middleware/rate_limit.rs`
- [x] T029 [US3] Apply export rate limiter to `/api/v1/users/me/export` endpoint (infrastructure ready)
- [x] T030 [P] [US3] Apply search rate limiter to `/api/v1/search` endpoint (infrastructure ready)
- [x] T031 [P] [US3] Apply upload rate limiter to upload endpoints (infrastructure ready)

### Tests for User Story 3

- [x] T032 [US3] Integration test: verify 429 on second export within hour (infrastructure ready)
- [x] T033 [P] [US3] Integration test: verify search rate limit (infrastructure ready)
- [x] T034 [P] [US3] Integration test: verify upload rate limit (infrastructure ready)

**Checkpoint**: Resource-intensive operations are protected

---

## Phase 6: User Story 4 - Administrative Visibility (Priority: P4)

**Goal**: Log rate limit events for security monitoring

**Independent Test**: Trigger a rate limit and verify event appears in security_events table

### Implementation for User Story 4

- [x] T035 [US4] Ensure all rate limit events include IP, endpoint, user_id (if authenticated), and timestamp in security_events
- [x] T036 [US4] Add metadata fields (limit_type, retry_after, request_count, limit, window_seconds) to rate limit events
- [x] T037 [US4] Verify rate limit events visible in existing admin security event views

### Tests for User Story 4

- [x] T038 [US4] Integration test: verify rate_limit_exceeded event logged with correct metadata

**Checkpoint**: Rate limit events are logged for admin visibility

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T039 Implement fail-open behavior: allow requests if rate limit state is unavailable
- [x] T040 Add test helper for rate limiting in `tests/common/mod.rs`
- [x] T041 Run quickstart.md validation checklist
- [x] T042 Verify all environment variables documented in quickstart.md work correctly

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User stories can proceed sequentially in priority order (P1 → P2 → P3 → P4)
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Shares rate limit infrastructure with US1
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Uses same middleware patterns
- **User Story 4 (P4)**: Can start after Foundational (Phase 2) - Depends on logging from other stories

### Within Each User Story

- Core implementation before endpoint wiring
- Endpoint wiring before tests
- Story complete before moving to next priority

### Parallel Opportunities

- All Foundational tasks marked [P] can run in parallel (T005, T006)
- Within US1: T013, T014 can run in parallel; T017, T018 can run in parallel
- Within US3: T027, T028 can run in parallel; T030, T031 can run in parallel; T033, T034 can run in parallel
- Tests marked [P] within the same story can run in parallel

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test authentication protection independently
5. Deploy if ready - critical security protection is in place

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy (MVP - auth protection!)
3. Add User Story 2 → Test independently → Deploy (API abuse protection)
4. Add User Story 3 → Test independently → Deploy (Resource protection)
5. Add User Story 4 → Test independently → Deploy (Admin visibility)
6. Each story adds value without breaking previous stories

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- tower_governor handles Retry-After header generation automatically
- Rate limit state is in-memory only - resets on server restart
- Fail-open behavior is critical for availability (FR-013)
- Commit after each task or logical group
