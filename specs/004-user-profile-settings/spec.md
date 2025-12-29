# Feature Specification: User Profile & Settings Management

**Feature Branch**: `004-user-profile-settings`
**Created**: 2025-12-29
**Status**: Draft
**Input**: User description: "I would like to display my profile and CRUD on all settings I can maintain as a user."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View My Profile (Priority: P1)

As a registered user, I want to view my complete profile information so that I can see how my profile appears and verify my account details are correct.

**Why this priority**: Viewing profile is the foundation for all other settings management - users must see current state before making changes.

**Independent Test**: Can be fully tested by logging in, navigating to profile page, and verifying all user information is displayed correctly.

**Acceptance Scenarios**:

1. **Given** I am logged in, **When** I navigate to my profile page, **Then** I see my display name, username, bio, avatar, measurement preference, and account creation date.
2. **Given** I am logged in, **When** I view my profile, **Then** I see my email address (partially masked for privacy) and email verification status.
3. **Given** I am not logged in, **When** I try to access my profile settings, **Then** I am redirected to the login page.

---

### User Story 2 - Edit Profile Information (Priority: P1)

As a registered user, I want to edit my profile information (display name, bio, avatar, measurement preference) so that I can personalize how I appear to other users.

**Why this priority**: Profile customization is essential for user identity and engagement on a social recipe-sharing platform.

**Independent Test**: Can be fully tested by logging in, editing each profile field, saving, and verifying changes persist.

**Acceptance Scenarios**:

1. **Given** I am on my profile settings page, **When** I update my display name and save, **Then** the new display name is shown on my profile and throughout the application.
2. **Given** I am on my profile settings page, **When** I update my bio (up to 500 characters) and save, **Then** the new bio is displayed on my public profile.
3. **Given** I am on my profile settings page, **When** I provide a new avatar URL and save, **Then** the new avatar is displayed on my profile.
4. **Given** I am on my profile settings page, **When** I change my measurement preference (metric/imperial), **Then** recipes display quantities in my preferred unit system.
5. **Given** I enter an invalid avatar URL, **When** I try to save, **Then** I see a validation error and the change is not saved.

---

### User Story 3 - Manage Email Address (Priority: P2)

As a registered user, I want to change my email address so that I can keep my contact information up to date.

**Why this priority**: Email is critical for account recovery and notifications, but less frequently changed than profile details.

**Independent Test**: Can be fully tested by initiating email change, confirming via email link, and verifying login works with new email.

**Acceptance Scenarios**:

1. **Given** I am on my account settings page, **When** I enter a new email address and my current password, **Then** a verification email is sent to the new address.
2. **Given** I have requested an email change, **When** I click the verification link in the email, **Then** my email is updated and I receive confirmation.
3. **Given** I enter an email that is already registered, **When** I try to change my email, **Then** I see an error that the email is unavailable.
4. **Given** I enter an incorrect current password, **When** I try to change my email, **Then** the request is rejected with an authentication error.

---

### User Story 4 - Change Password (Priority: P2)

As a registered user, I want to change my password so that I can maintain account security.

**Why this priority**: Security feature that users expect but don't use frequently.

**Independent Test**: Can be fully tested by changing password and verifying new password works for login.

**Acceptance Scenarios**:

1. **Given** I am on my security settings page, **When** I enter my current password and a new valid password (twice), **Then** my password is updated.
2. **Given** I enter a weak new password, **When** I try to save, **Then** I see password strength requirements and the change is rejected.
3. **Given** I enter an incorrect current password, **When** I try to change password, **Then** the request is rejected.
4. **Given** I change my password successfully, **When** I log out and log back in, **Then** I can authenticate with the new password.
5. **Given** I change my password successfully, **When** I have other active sessions on other devices, **Then** those sessions are immediately invalidated.

---

### User Story 5 - Manage Two-Factor Authentication (Priority: P2)

As a registered user, I want to enable, disable, and manage two-factor authentication so that I can secure my account.

**Why this priority**: Important security feature, but not required for basic profile functionality.

**Independent Test**: Can be fully tested by enabling 2FA, verifying login requires code, and disabling 2FA.

**Acceptance Scenarios**:

1. **Given** 2FA is disabled, **When** I initiate 2FA setup, **Then** I see a QR code and secret key to add to my authenticator app.
2. **Given** I have scanned the QR code, **When** I enter a valid code from my authenticator, **Then** 2FA is enabled and I receive recovery codes.
3. **Given** 2FA is enabled, **When** I want to disable it, **Then** I must enter my password and a valid 2FA code to confirm.
4. **Given** 2FA is enabled, **When** I view my 2FA status, **Then** I see how many recovery codes remain.
5. **Given** 2FA is enabled, **When** I request new recovery codes, **Then** old codes are invalidated and new codes are generated.

---

### User Story 6 - Manage Active Sessions (Priority: P3)

As a registered user, I want to view and revoke my active login sessions so that I can ensure no unauthorized access to my account.

**Why this priority**: Security housekeeping feature, less critical than core profile and authentication.

**Independent Test**: Can be fully tested by viewing sessions list and revoking a session.

**Acceptance Scenarios**:

1. **Given** I am on my security settings page, **When** I view active sessions, **Then** I see a list of devices/browsers with login dates and locations.
2. **Given** I have multiple active sessions, **When** I revoke a specific session, **Then** that session is immediately invalidated.
3. **Given** I revoke a session, **When** that device tries to access protected resources, **Then** they are redirected to login.
4. **Given** I am viewing sessions, **Then** I cannot revoke my current session (must use logout instead).

---

### User Story 7 - Request Account Deletion (Priority: P3)

As a registered user, I want to request deletion of my account so that I can remove my data from the platform.

**Why this priority**: Important for user rights and privacy compliance, but infrequent operation.

**Independent Test**: Can be fully tested by requesting deletion, verifying grace period, and canceling or completing deletion.

**Acceptance Scenarios**:

1. **Given** I am on my account settings page, **When** I request account deletion and confirm with my password, **Then** my account is scheduled for deletion after a grace period.
2. **Given** my account is scheduled for deletion, **When** I log in during the grace period, **Then** I see a warning banner with the deletion date and option to cancel.
3. **Given** my account is scheduled for deletion, **When** I cancel the deletion request, **Then** my account returns to normal status.
4. **Given** the grace period has passed, **When** the system processes deletions, **Then** my account and personal data are permanently removed.

---

### Edge Cases

- What happens when a user tries to update their profile with an empty display name? → Rejected with validation error (minimum 1 character).
- How does the system handle concurrent profile updates from multiple sessions? → Last write wins; user sees current state on next load.
- What happens if email verification link expires? → User can request a new verification email.
- How long is the account deletion grace period? → 30 days (industry standard).
- What happens to user's recipes and books upon account deletion? → User explicitly chooses during deletion request: either anonymize content (replace author with "Deleted User") or delete all content permanently.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST display the authenticated user's complete profile information on a dedicated profile page.
- **FR-002**: System MUST allow users to update their display name (1-100 characters).
- **FR-003**: System MUST allow users to update their bio (0-500 characters).
- **FR-004**: System MUST allow users to update their avatar via URL (validated as proper URL format).
- **FR-005**: System MUST allow users to change their measurement preference between metric and imperial.
- **FR-006**: System MUST require current password verification for security-sensitive changes (email, password, 2FA, account deletion).
- **FR-007**: System MUST send verification email when user requests email change.
- **FR-008**: System MUST validate new passwords meet strength requirements (minimum 8 characters, mixed case, numbers, special characters).
- **FR-009**: System MUST provide 2FA setup via TOTP-compatible authenticator apps.
- **FR-010**: System MUST generate and display recovery codes when 2FA is enabled.
- **FR-011**: System MUST display list of active sessions with device/browser information.
- **FR-012**: System MUST allow users to revoke any session except the current one.
- **FR-013**: System MUST implement a 30-day grace period for account deletion requests.
- **FR-014**: System MUST allow cancellation of deletion request during the grace period.
- **FR-015**: System MUST mask sensitive information (partial email display) appropriately.
- **FR-016**: System MUST present users with an explicit choice during account deletion: anonymize content (retain with "Deleted User" attribution) or delete all content permanently.
- **FR-017**: System MUST invalidate all other sessions (except current) when a user changes their password.

### Key Entities

- **User Profile**: Display name, username, bio, avatar URL, measurement preference, creation date - public-facing identity.
- **User Account**: Email, password, email verification status, 2FA settings - private security information.
- **Session**: Device/browser info, IP address, login timestamp, last activity - login tracking.
- **Deletion Request**: Request timestamp, scheduled deletion date, cancellation status - account lifecycle.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view and update all profile fields within 30 seconds.
- **SC-002**: Password changes take effect immediately for all subsequent login attempts.
- **SC-003**: 2FA setup completion rate exceeds 80% for users who initiate the process.
- **SC-004**: Session revocation takes effect within 5 seconds across all user devices.
- **SC-005**: 100% of email change requests result in verification email delivery within 2 minutes.
- **SC-006**: Account deletion grace period notifications are shown on every login during the 30-day window.
- **SC-007**: Users can complete any single settings change in under 3 clicks/taps from the settings page.

## Clarifications

### Session 2025-12-29

- Q: How should user content (recipes, books) be handled on account deletion? → A: User chooses during deletion: keep content (anonymized) or delete all.
- Q: Should existing sessions be invalidated after password change? → A: Invalidate all other sessions (keep only current session).

## Assumptions

- User authentication (login/logout) is already implemented (feature 002-user-auth).
- Email delivery infrastructure is in place and functional.
- TOTP 2FA infrastructure exists (referenced in current User model).
- Session management is already implemented with the ability to list and revoke sessions.
- The existing User model fields (display_name, bio, avatar_url, measurement_pref) are used.
