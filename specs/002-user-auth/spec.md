# Feature Specification: User Authentication

**Feature Branch**: `002-user-auth`
**Created**: 2025-12-26
**Status**: In Progress (MVP Complete - User Stories 1 & 2)
**Input**: User description: "Lets go for authentication, it should be rock solid, easy for end user. Everything in code related to auth now is a stub, feel free to remove everything."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - New User Registration (Priority: P1)

A visitor discovers Oppskrift and wants to create an account to save recipes and follow other cooks. They should be able to register quickly with minimal friction while ensuring their account is secure.

**Why this priority**: Without registration, no other authenticated features work. This is the gateway to the entire platform.

**Independent Test**: Can be fully tested by registering a new account and verifying email confirmation. Delivers immediate value by enabling account creation.

**Acceptance Scenarios**:

1. **Given** a visitor on the registration page, **When** they enter a valid email, username, and password, **Then** they receive a confirmation email and see a "check your email" message
2. **Given** a visitor with a pending confirmation, **When** they click the confirmation link in their email, **Then** their account is activated and they are logged in
3. **Given** a visitor attempting registration, **When** they enter an email already in use, **Then** they see a clear error message without revealing if the email exists (security)
4. **Given** a visitor attempting registration, **When** they enter a weak password, **Then** they see specific feedback about password requirements
5. **Given** a confirmation link, **When** it is older than 24 hours, **Then** clicking it shows an expiration message with option to request a new link

---

### User Story 2 - Returning User Login (Priority: P1)

A registered user returns to Oppskrift and wants to access their saved recipes and activity feed. They should be able to log in quickly and securely.

**Why this priority**: Login is essential for any authenticated session. Equal priority with registration as both are required for a functional auth system.

**Independent Test**: Can be fully tested by logging in with valid credentials and accessing protected content.

**Acceptance Scenarios**:

1. **Given** a registered user on the login page, **When** they enter correct email and password, **Then** they are logged in and redirected to their dashboard
2. **Given** a user attempting login, **When** they enter incorrect credentials, **Then** they see a generic "invalid credentials" message (no hint about which field is wrong)
3. **Given** a user who has failed login 5 times, **When** they attempt another login, **Then** they are temporarily locked out with a clear message about when they can retry
4. **Given** a logged-in user, **When** they close the browser and return within 7 days, **Then** they remain logged in (remember me by default)
5. **Given** a logged-in user on a shared computer, **When** they click "Log out", **Then** their session is terminated and they cannot access protected pages

---

### User Story 3 - Password Recovery (Priority: P2)

A user forgets their password and needs to regain access to their account without losing any data or settings.

**Why this priority**: Critical for user retention but not needed for initial account setup. Users will need this after launch.

**Independent Test**: Can be fully tested by requesting password reset, receiving email, and setting new password.

**Acceptance Scenarios**:

1. **Given** a user on the forgot password page, **When** they enter their registered email, **Then** they receive a password reset email and see a confirmation message
2. **Given** a user on the forgot password page, **When** they enter an unregistered email, **Then** they see the same confirmation message (no email enumeration)
3. **Given** a user with a reset link, **When** they click it within 1 hour, **Then** they can set a new password
4. **Given** a user with a reset link, **When** they click it after 1 hour, **Then** they see an expiration message with option to request a new link
5. **Given** a user who just reset their password, **When** they try to use the old password, **Then** login fails with generic error

---

### User Story 4 - Account Security Settings (Priority: P3)

A security-conscious user wants to manage their account security by changing their password, viewing active sessions, and optionally enabling TOTP-based two-factor authentication.

**Why this priority**: Important for user trust and security but not blocking for basic functionality.

**Independent Test**: Can be fully tested by accessing security settings and changing password.

**Acceptance Scenarios**:

1. **Given** a logged-in user in account settings, **When** they change their password (providing current password), **Then** the password is updated and all other sessions are terminated
2. **Given** a logged-in user in account settings, **When** they view active sessions, **Then** they see a list of devices/browsers with last activity time
3. **Given** a logged-in user viewing sessions, **When** they revoke a session, **Then** that session is immediately terminated
4. **Given** a user whose password was changed, **When** they are on another device, **Then** they are logged out and must re-authenticate
5. **Given** a logged-in user in security settings, **When** they enable 2FA, **Then** they see a QR code to scan with an authenticator app and must confirm with a valid code
6. **Given** a user with 2FA enabled, **When** they log in with correct password, **Then** they are prompted for their 6-digit TOTP code before access is granted
7. **Given** a user with 2FA enabled, **When** they disable 2FA, **Then** they must confirm with their password and a valid TOTP code

---

### User Story 5 - Account Deletion (Priority: P3)

A user decides to leave the platform and wants to delete their account and all associated data.

**Why this priority**: Required for GDPR compliance but not blocking for launch.

**Independent Test**: Can be fully tested by initiating account deletion and verifying data removal.

**Acceptance Scenarios**:

1. **Given** a logged-in user in account settings, **When** they request account deletion, **Then** they must confirm with their password
2. **Given** a user who confirmed deletion, **When** the grace period (7 days) passes, **Then** their account and personal data are permanently deleted
3. **Given** a user who requested deletion, **When** they log in during the grace period, **Then** they can cancel the deletion request
4. **Given** a deleted user's recipes, **When** they were public, **Then** they are either anonymized or deleted based on user's choice during deletion

---

### Edge Cases

- What happens when a user tries to register with a disposable email domain? System accepts it (no blocking of email providers)
- What happens when a user's session expires mid-action? They are redirected to login with a message, then returned to their previous page
- How does the system handle concurrent login attempts? Rate limiting applies per IP and per account
- What happens if email delivery fails? User can request resend; system logs delivery failures for monitoring
- What happens when a user changes their email address? New email must be confirmed before it takes effect; old email remains active until confirmation

## Requirements *(mandatory)*

### Functional Requirements

**Registration**
- **FR-001**: System MUST allow users to register with email, username, and password
- **FR-002**: System MUST require email confirmation before account activation
- **FR-003**: System MUST enforce password requirements: minimum 10 characters, at least one uppercase, one lowercase, and one number
- **FR-004**: System MUST check passwords against a list of commonly breached passwords
- **FR-005**: System MUST validate username: alphanumeric and underscores only (a-z, 0-9, _), 3-30 characters, stored lowercase, unique (case-insensitive), and not in reserved words list (admin, root, system, support, help, oppskrift)
- **FR-006**: System MUST validate email format and uniqueness (case-insensitive)
- **FR-007**: Confirmation links MUST expire after 24 hours

**Authentication**
- **FR-008**: System MUST authenticate users via email and password
- **FR-009**: System MUST hash passwords using a secure, modern algorithm (not plaintext or weak hashing)
- **FR-010**: System MUST implement rate limiting: max 5 failed attempts per account, then 15-minute lockout
- **FR-011**: System MUST implement rate limiting: max 10 login attempts per IP per minute
- **FR-012**: System MUST maintain secure sessions with configurable expiration (default 7 days)
- **FR-013**: System MUST invalidate all sessions when password is changed
- **FR-014**: Login error messages MUST NOT reveal whether email or password was incorrect

**Password Recovery**
- **FR-015**: System MUST allow password reset via email
- **FR-016**: Password reset links MUST expire after 1 hour
- **FR-017**: Password reset links MUST be single-use
- **FR-018**: System MUST NOT reveal whether an email is registered during password reset

**Session Management**
- **FR-019**: Users MUST be able to view their active sessions
- **FR-020**: Users MUST be able to revoke individual sessions
- **FR-021**: Users MUST be able to log out, terminating their current session
- **FR-022**: System MUST track session metadata: device type, browser, last activity, IP (for display only)

**Account Management**
- **FR-023**: Users MUST be able to change their password (requiring current password)
- **FR-024**: Users MUST be able to change their email (requiring confirmation of new email)
- **FR-025**: Users MUST be able to request account deletion
- **FR-026**: Account deletion MUST have a 7-day grace period with ability to cancel
- **FR-027**: System MUST log all security-relevant events (login, logout, password change, failed attempts)

**Two-Factor Authentication (P3)**
- **FR-028**: Users MUST be able to enable TOTP-based 2FA via authenticator apps
- **FR-029**: System MUST display QR code and manual entry key during 2FA setup
- **FR-030**: System MUST require valid TOTP code confirmation before 2FA is activated
- **FR-031**: Users with 2FA enabled MUST provide valid TOTP code after password during login
- **FR-032**: Users MUST be able to disable 2FA (requiring password and valid TOTP code)
- **FR-033**: System MUST provide one-time recovery codes when 2FA is enabled (for account recovery if device is lost)

### Key Entities

- **User**: Represents a registered account with email, username, hashed password, email confirmation status, 2FA status, TOTP secret (if enabled), creation date, and federation settings
- **Recovery Code**: One-time use codes for 2FA account recovery, with user reference, hashed code, and used status
- **Session**: Represents an active login with user reference, token, device info, IP address, creation time, last activity, and expiration
- **Password Reset Token**: Represents a pending password reset with user reference, token, creation time, expiration, and used status
- **Email Confirmation Token**: Represents a pending email confirmation with user reference, token, email address, creation time, and expiration
- **Security Event**: Represents an auditable security action with user reference, event type, IP address, timestamp, and metadata

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can complete registration (including email confirmation) in under 3 minutes
- **SC-002**: Users can log in within 10 seconds of entering credentials
- **SC-003**: Password reset flow completes in under 5 minutes (including email delivery)
- **SC-004**: 95% of registration attempts succeed on first try (excluding intentional validation failures)
- **SC-005**: Zero passwords stored in recoverable format (verified by security audit)
- **SC-006**: System blocks brute force attempts: no successful login after 5 failed attempts within lockout period
- **SC-007**: All security events are logged with sufficient detail for incident investigation
- **SC-008**: Session revocation takes effect within 1 minute across all user devices
- **SC-009**: Account deletion completes data removal within 24 hours after grace period
- **SC-010**: System handles 1000 concurrent authentication requests without degradation

## Clarifications

### Session 2025-12-26

- Q: Should the system support social login (OAuth providers like Google, GitHub)? → A: No, email/password only - keeps auth simple, auditable, and under full control
- Q: What format rules apply to usernames? → A: Alphanumeric + underscores (a-z, 0-9, _), 3-30 characters, stored lowercase
- Q: Is two-factor authentication in scope? → A: Yes, TOTP-based 2FA (authenticator apps) as P3 optional feature
- Q: Should the system block reserved usernames? → A: Yes, block common reserved words (admin, root, system, support, help, oppskrift)

## Assumptions

- Email delivery infrastructure is available and reliable
- Social login (OAuth) is explicitly out of scope; email/password is the sole authentication method
- Users have access to their registered email for confirmation and recovery
- The application already has HTTPS configured for all traffic
- Rate limiting can be implemented at the application level
- Session storage can handle the expected number of concurrent users
- GDPR compliance is required (hence the account deletion grace period and data removal requirements)
