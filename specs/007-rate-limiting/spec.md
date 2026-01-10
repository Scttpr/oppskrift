# Feature Specification: Rate Limiting

**Feature Branch**: `007-rate-limiting`
**Created**: 2026-01-10
**Status**: Draft
**Input**: User description: "implement full rate limiting"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Protected Authentication Endpoints (Priority: P1)

As a platform operator, I want authentication endpoints (login, registration, password reset) to be protected against brute force attacks so that user accounts remain secure.

**Why this priority**: Authentication endpoints are the primary attack vector for malicious actors. Protecting these first provides the highest security value and protects all user accounts.

**Independent Test**: Can be fully tested by attempting multiple rapid login requests and verifying that excessive requests are blocked, delivering immediate security protection.

**Acceptance Scenarios**:

1. **Given** a user attempts to login, **When** they exceed 5 failed attempts within 15 minutes from the same IP, **Then** subsequent login attempts are blocked for 15 minutes with a clear error message
2. **Given** a user is rate limited on login, **When** the cooldown period expires, **Then** they can attempt to login again
3. **Given** multiple IPs attempt to login to the same account, **When** combined attempts exceed the threshold, **Then** the account is temporarily locked regardless of IP
4. **Given** a rate limit is triggered, **When** the user receives the error response, **Then** they see a user-friendly message indicating how long to wait

---

### User Story 2 - Protected API Endpoints (Priority: P2)

As a platform operator, I want general API endpoints to be protected against abuse so that the system remains responsive for all users.

**Why this priority**: After authentication, general API abuse is the next most impactful attack vector. This protects system resources and ensures fair access for all users.

**Independent Test**: Can be fully tested by making rapid API requests and verifying that excessive requests receive a "too many requests" response.

**Acceptance Scenarios**:

1. **Given** an authenticated user makes API requests, **When** they exceed 100 requests per minute, **Then** additional requests receive a 429 status with retry information
2. **Given** an unauthenticated user makes API requests, **When** they exceed 30 requests per minute from the same IP, **Then** additional requests are blocked
3. **Given** a user is rate limited, **When** they check the response headers, **Then** they see when they can retry (Retry-After header)

---

### User Story 3 - Protected Resource-Intensive Operations (Priority: P3)

As a platform operator, I want expensive operations (data export, bulk operations, search) to have stricter limits so that system resources are preserved.

**Why this priority**: Resource-intensive operations can be weaponized to cause denial of service. Protecting these ensures system stability under load.

**Independent Test**: Can be fully tested by requesting multiple data exports and verifying that excessive requests are blocked with appropriate messaging.

**Acceptance Scenarios**:

1. **Given** a user requests a data export, **When** they have already exported within the last hour, **Then** the request is blocked with a message indicating the cooldown period
2. **Given** a user performs bulk search operations, **When** they exceed 10 searches per minute, **Then** additional searches are throttled
3. **Given** a user uploads multiple images, **When** they exceed 20 uploads per 5 minutes, **Then** additional uploads are blocked temporarily

---

### User Story 4 - Administrative Visibility (Priority: P4)

As a platform administrator, I want to see rate limiting activity so that I can identify abuse patterns and adjust limits as needed.

**Why this priority**: While not critical for protection, visibility enables tuning and incident response.

**Independent Test**: Can be fully tested by triggering rate limits and verifying that events appear in security logs.

**Acceptance Scenarios**:

1. **Given** a rate limit is triggered, **When** an administrator views security events, **Then** they see the rate limit event with IP, endpoint, and timestamp
2. **Given** an IP is repeatedly rate limited, **When** an administrator reviews the logs, **Then** they can identify the pattern across multiple endpoints

---

### Edge Cases

- What happens when a user is behind a shared IP (NAT, corporate network)? Rate limits should be reasonable enough to not block legitimate shared usage.
- How does the system handle distributed attacks from many IPs? Account-level rate limiting supplements IP-based limiting.
- What happens when rate limit state storage fails? The system should fail open (allow requests) rather than blocking all traffic.
- How are WebSocket connections handled? Long-lived connections should have message-rate limits, not connection-rate limits.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST rate limit failed authentication attempts to 5 per 15 minutes per IP address
- **FR-002**: System MUST rate limit failed authentication attempts to 10 per hour per user account (across all IPs)
- **FR-003**: System MUST rate limit general API requests to 100 per minute for authenticated users
- **FR-004**: System MUST rate limit general API requests to 30 per minute for unauthenticated users (per IP)
- **FR-005**: System MUST rate limit data export operations to 1 per hour per user
- **FR-006**: System MUST rate limit search operations to 10 per minute per user
- **FR-007**: System MUST rate limit file uploads to 20 per 5 minutes per user
- **FR-008**: System MUST return HTTP 429 (Too Many Requests) status when rate limits are exceeded
- **FR-009**: System MUST include Retry-After header in rate limit responses indicating seconds until retry
- **FR-010**: System MUST display user-friendly error messages explaining the rate limit and wait time
- **FR-011**: System MUST log all rate limit events with IP address, endpoint, user (if authenticated), and timestamp
- **FR-012**: System MUST allow rate limits to be configured without code changes
- **FR-013**: System MUST fail open if rate limit storage is unavailable (allow requests rather than block all)

### Key Entities

- **Rate Limit Rule**: Defines the limit threshold, time window, and scope (IP, user, endpoint) for a category of requests
- **Rate Limit Event**: Records when a rate limit was triggered, including the blocked request details
- **Rate Limit Counter**: Tracks current request counts within time windows for enforcement

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Brute force login attempts are blocked within 5 failed attempts, preventing account compromise
- **SC-002**: System maintains responsiveness for legitimate users even when under rate-limited attack conditions
- **SC-003**: 95% of rate-limited users see a clear message explaining why their request was blocked and when to retry
- **SC-004**: Rate limit events are logged within 1 second of occurrence for security monitoring
- **SC-005**: Legitimate users on shared networks (up to 50 users per IP) can use the platform without being blocked
- **SC-006**: System handles rate limit state failures gracefully without blocking all traffic
- **SC-007**: Rate limit checks complete in sub-millisecond time (<1ms) to avoid perceptible user delay

## Clarifications

### Session 2026-01-10

- Q: How should proxy/load balancer headers be trusted for IP extraction? → A: Trust only first hop from configured trusted proxies (secure, common setup)
- Q: What is the latency target for rate limit checks? → A: Sub-millisecond (<1ms), requires in-memory storage

## Assumptions

- The platform already has session-based authentication in place
- Security events table exists for logging rate limit events
- The existing `tower_governor` dependency will be used for implementation
- IP addresses are extracted using trusted proxy configuration: only the first hop from explicitly configured trusted proxy IPs is used (prevents X-Forwarded-For spoofing)
- Rate limit configuration will be managed via environment variables
- In-memory rate limiting is required to meet sub-millisecond latency target; distributed deployments may require external state storage with relaxed latency requirements in the future

## Dependencies

- Existing authentication middleware for user identification
- Existing security events logging infrastructure
- IP extraction from request headers (X-Forwarded-For handling)

## Out of Scope

- Distributed rate limiting across multiple server instances (future enhancement)
- Automatic IP blocking/banning based on repeated violations
- CAPTCHA challenges as an alternative to hard blocking
- Rate limit management UI for administrators
- Per-user rate limit customization (premium users with higher limits)
