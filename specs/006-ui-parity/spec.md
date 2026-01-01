# Feature Specification: UI Parity

**Feature Branch**: `006-ui-parity`
**Created**: 2026-01-01
**Status**: Draft
**Input**: User description: "Close gaps between API and UI features"

## Clarifications

### Session 2026-01-01

- Q: How long should security events be retained? → A: 90 days - standard security audit window
- Q: What threshold triggers async data export? → A: 50 recipes - async earlier, safer for server
- Q: What happens when a contribution is rejected? → A: Notify contributor with optional reason, allow re-submission

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View and Manage Sessions (Priority: P1)

A user wants to see all active sessions (devices/browsers) where they are logged in and revoke access to any session they don't recognize or no longer need.

**Why this priority**: Security-critical feature. Users must be able to detect unauthorized access and terminate suspicious sessions immediately.

**Independent Test**: Can be fully tested by logging in from multiple browsers, viewing the sessions list, and revoking one session to verify it's terminated.

**Acceptance Scenarios**:

1. **Given** a user is logged in from multiple devices, **When** they visit the sessions page, **Then** they see a list of all active sessions with device info and last activity time
2. **Given** a user views their sessions, **When** they click "Revoke" on a specific session, **Then** that session is immediately terminated and removed from the list
3. **Given** a user has only one active session (current), **When** they view sessions, **Then** the current session is marked and cannot be revoked

---

### User Story 2 - View Security Event History (Priority: P1)

A user wants to review their account's security events (logins, password changes, 2FA changes) to detect any suspicious activity.

**Why this priority**: Essential for security awareness. Users need visibility into account activity to detect compromise early.

**Independent Test**: Can be tested by performing security-relevant actions (login, password change), then viewing the security events page to verify they appear.

**Acceptance Scenarios**:

1. **Given** a user has performed various account actions, **When** they visit the security events page, **Then** they see a chronological list of security events
2. **Given** security events exist, **When** viewing the list, **Then** each event shows type, timestamp, IP address, and device information
3. **Given** many security events exist, **When** viewing the page, **Then** events are paginated with the most recent first

---

### User Story 3 - Toggle Federation Settings (Priority: P2)

A user wants to control whether their profile and recipes are discoverable and shareable via ActivityPub federation with other instances.

**Why this priority**: Privacy control. Users should decide whether their content is federated to other servers.

**Independent Test**: Can be tested by toggling federation on/off and verifying the user's ActivityPub endpoint returns appropriate responses.

**Acceptance Scenarios**:

1. **Given** a user is on the settings page, **When** they view privacy settings, **Then** they see a clear toggle for federation with explanation of what it means
2. **Given** federation is enabled, **When** the user disables it, **Then** their profile is no longer discoverable via WebFinger or ActivityPub
3. **Given** federation is disabled, **When** the user enables it, **Then** their public content becomes federated

---

### User Story 4 - Export Personal Data (Priority: P2)

A user wants to download all their personal data (profile, recipes, books, activity) in a portable format for backup or migration purposes.

**Why this priority**: GDPR compliance and user autonomy. Users must be able to retrieve their data.

**Independent Test**: Can be tested by clicking the export button and verifying the downloaded file contains all expected user data.

**Acceptance Scenarios**:

1. **Given** a user is on the account settings page, **When** they click "Export My Data", **Then** a download is initiated containing their data
2. **Given** export is requested, **When** the download completes, **Then** the file is in a standard format containing profile, recipes, books, and activity history
3. **Given** the user has extensive data, **When** they request export, **Then** they see a progress indicator and the system handles large exports gracefully

---

### User Story 5 - Manage Book Contributions (Priority: P2)

A book owner wants to view, accept, and remove recipe contributions from collaborators. Contributors want to see pending and accepted contributions.

**Why this priority**: Enables collaborative recipe books. Without this UI, the contribution API is unusable.

**Independent Test**: Can be tested by having a contributor add a recipe to a book, then having the owner view and manage the contribution.

**Acceptance Scenarios**:

1. **Given** a book has contributions, **When** the owner views the book, **Then** they see a "Contributions" section listing all contributed recipes
2. **Given** pending contributions exist, **When** the owner reviews them, **Then** they can accept or reject each contribution
3. **Given** a contribution is accepted, **When** viewing the book, **Then** the recipe appears in the book with contributor attribution
4. **Given** a user is a contributor, **When** they view their contributions, **Then** they see status of their submitted recipes

---

### User Story 6 - View Followers and Following Lists (Priority: P3)

A user wants to see who follows them and who they follow, and manage these relationships.

**Why this priority**: Standard social feature. Users expect to see and manage their social connections.

**Independent Test**: Can be tested by following users, then viewing the followers/following pages to verify the lists are accurate.

**Acceptance Scenarios**:

1. **Given** a user has followers, **When** they visit their profile's followers page, **Then** they see a list of all followers with profile previews
2. **Given** a user follows others, **When** they visit their following page, **Then** they see everyone they follow with unfollow options
3. **Given** viewing another user's profile, **When** clicking their follower/following counts, **Then** the user sees those lists (for public profiles)

---

### Edge Cases

- What happens when a user tries to revoke their only/current session?
- How does the system handle data export for users with thousands of recipes?
- What happens to federation toggle when the user has pending outgoing activities?
- How are contributions displayed when a book has reached capacity limits?
- What happens when viewing followers/following for a deleted or suspended user?

## Requirements *(mandatory)*

### Functional Requirements

**Sessions Management**
- **FR-001**: System MUST display all active sessions for the authenticated user
- **FR-002**: System MUST show session details: device type, browser, IP address, and last activity
- **FR-003**: System MUST allow users to revoke any session except their current one
- **FR-004**: System MUST provide a "Revoke all other sessions" action

**Security Events**
- **FR-005**: System MUST display security events in reverse chronological order
- **FR-006**: System MUST show event type, timestamp, IP address, and device for each event
- **FR-007**: System MUST paginate events (20 per page)
- **FR-008**: Security events page MUST be accessible only to the authenticated user
- **FR-025**: System MUST retain security events for 90 days, after which they are automatically purged

**Federation Toggle**
- **FR-009**: System MUST provide a toggle to enable/disable ActivityPub federation
- **FR-010**: System MUST explain the implications of federation in user-friendly language
- **FR-011**: Disabling federation MUST stop serving ActivityPub endpoints for the user

**Data Export**
- **FR-012**: System MUST provide a "Download My Data" button in account settings
- **FR-013**: Export MUST include: profile info, recipes, books, followers/following, activity history
- **FR-014**: Export format MUST be machine-readable (JSON)
- **FR-015**: System MUST handle exports asynchronously when user has more than 50 recipes; smaller exports download immediately

**Book Contributions**
- **FR-016**: Book view MUST show a contributions section for owners and contributors
- **FR-017**: Owners MUST be able to view all contributions with status
- **FR-018**: Owners MUST be able to accept or reject pending contributions
- **FR-019**: Contributors MUST see the status of their submitted recipes
- **FR-020**: Accepted contributions MUST display contributor attribution
- **FR-026**: When rejecting a contribution, owners MAY provide an optional reason
- **FR-027**: Rejected contributions MUST be visible to the contributor with status and reason (if provided); formal push/email notifications are out of scope
- **FR-028**: Contributors MUST be able to re-submit a previously rejected recipe to the same book

**Followers/Following**
- **FR-021**: System MUST provide a page listing all followers for a user
- **FR-022**: System MUST provide a page listing all users being followed
- **FR-023**: Each list item MUST include user avatar, display name, and follow/unfollow action
- **FR-024**: Lists MUST be paginated for users with many connections

### Key Entities

- **Session**: Represents an active login session with device metadata, IP, creation time, and last activity
- **SecurityEvent**: Audit log entry for security-relevant actions (login, password change, 2FA, etc.)
- **Contribution**: A recipe submitted to a book by a non-owner, with pending/accepted/rejected status
- **FollowRelationship**: Connection between two users representing a follow action

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view and revoke sessions within 10 seconds of navigation
- **SC-002**: Security events page loads with 50+ events in under 2 seconds
- **SC-003**: 100% of API features covered in this scope have corresponding UI access
- **SC-004**: Users can complete data export initiation in under 3 clicks from settings
- **SC-005**: Book contribution management achievable without leaving the book view page
- **SC-006**: Followers/following lists display up to 100 users without pagination required

## Assumptions

- Session data already includes device/browser information from user-agent parsing
- Security events are already being logged via the existing security_events table
- The existing API endpoints are stable and require no changes
- Data export can reuse existing JSON serialization from ActivityPub representations
- Contribution workflow follows a simple submit-accept/reject model without revision requests

## Scope Boundaries

**In Scope**:
- Sessions management UI (view, revoke individual, revoke all others)
- Security events viewer with pagination
- Federation on/off toggle in settings
- Data export trigger and download
- Book contributions management UI
- Followers and following list pages

**Out of Scope**:
- Email notifications for security events
- Scheduled/automatic data exports
- Contribution revision or editing workflow
- Push/email notifications for contribution status changes (contributor views status on book page per FR-019)
- Blocking/muting users from follower lists
- Session timeout configuration by user
