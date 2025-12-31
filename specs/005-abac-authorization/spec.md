# Feature Specification: ABAC Authorization System

**Feature Branch**: `005-abac-authorization`
**Created**: 2025-12-30
**Status**: Draft
**Input**: User description: "I would like to implement a full ABAC system that scales for future knowing that it should be possible to have access to our own thing, to share publicly, to share with certain people and to share with certain instances and to share with certain groups of people."

## Clarifications

### Session 2025-12-30

- Q: Should inaccessible resources return 404 or 403? → A: Always return 404 Not Found (hides resource existence)
- Q: Should permissions support expiration dates for temporary sharing? → A: No, all permissions are permanent until explicitly revoked
- Q: Does sharing a book grant access to recipes in it? → A: Yes, sharing a book automatically grants access to all recipes in that book
- Q: Should books support collaborative editing by multiple users? → A: Yes, via "contributor" permission level; contributors can add their own recipes to the book while maintaining individual recipe ownership

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Owner Full Control (Priority: P1)

As a resource owner (recipe or book), I have full control over my content including the ability to view, edit, delete, and manage who can access it.

**Why this priority**: This is the foundation of all access control. Without owner permissions working correctly, no other sharing scenarios can function. This represents the most common use case where users manage their own content.

**Independent Test**: Can be fully tested by creating a recipe/book as a user and verifying complete CRUD operations work only for the owner. Delivers immediate value for basic content management.

**Acceptance Scenarios**:

1. **Given** I am logged in and have created a recipe, **When** I view my recipes list, **Then** I see all my recipes regardless of visibility settings
2. **Given** I own a recipe, **When** I attempt to edit it, **Then** the changes are saved successfully
3. **Given** I own a recipe, **When** I attempt to delete it, **Then** it is removed and no longer accessible
4. **Given** I own a recipe, **When** another user attempts to edit or delete it, **Then** they receive a forbidden response
5. **Given** I own a book, **When** I add or remove recipes from it, **Then** the changes are applied

---

### User Story 2 - Public Sharing (Priority: P1)

As a resource owner, I can make my recipes and books publicly visible so that anyone (including unauthenticated users) can discover and view them.

**Why this priority**: Public sharing enables content discovery and is essential for a recipe-sharing platform. This is the primary way users will grow their audience and share culinary knowledge.

**Independent Test**: Can be tested by setting a recipe to public visibility and verifying unauthenticated users can view it. Delivers value for content creators who want to share recipes.

**Acceptance Scenarios**:

1. **Given** I own a recipe, **When** I set its visibility to public, **Then** anyone can view the recipe details
2. **Given** a recipe is public, **When** an unauthenticated user requests it, **Then** they can view the full recipe
3. **Given** I own a recipe set to private, **When** an unauthenticated user requests it, **Then** they receive a not found or forbidden response
4. **Given** a book is public, **When** any user views the book, **Then** they see all public recipes in the book

---

### User Story 3 - Private Content (Priority: P1)

As a resource owner, I can keep my recipes and books private so that only I can access them.

**Why this priority**: Privacy is fundamental for users who want to draft content before publishing or keep personal recipes private. This protects user content from unauthorized access.

**Independent Test**: Can be tested by creating a private recipe and verifying no other user can access it. Delivers value for users who need content privacy.

**Acceptance Scenarios**:

1. **Given** I own a private recipe, **When** I view it, **Then** I can see all recipe details
2. **Given** a recipe is private, **When** another authenticated user requests it, **Then** they receive a not found or forbidden response
3. **Given** I create a new recipe without specifying visibility, **When** the recipe is saved, **Then** it defaults to private visibility
4. **Given** I own a private book, **When** other users browse books, **Then** my private book does not appear in their results

---

### User Story 4 - Share with Specific Users (Priority: P2)

As a resource owner, I can share my recipe or book with specific users, granting them either view or edit access.

**Why this priority**: Enables collaboration between specific users without making content public. Essential for household recipe sharing, cooking club collaborations, or trusted friend circles.

**Independent Test**: Can be tested by sharing a recipe with a specific user and verifying they can access it while others cannot. Delivers value for personal sharing.

**Acceptance Scenarios**:

1. **Given** I own a private recipe, **When** I share it with another user for viewing, **Then** that user can view the recipe
2. **Given** I own a recipe shared with a user for editing, **When** that user edits the recipe, **Then** their changes are saved
3. **Given** I own a recipe shared with specific users, **When** a non-shared user requests it, **Then** they receive forbidden or not found
4. **Given** I shared a recipe with a user, **When** I revoke their access, **Then** they can no longer view the recipe
5. **Given** I am a recipient of a shared recipe, **When** I view my shared content, **Then** I can see who shared it with me

---

### User Story 5 - Followers Access (Priority: P2)

As a resource owner, I can share content with all my followers, allowing anyone who follows me to access specific recipes or books.

**Why this priority**: Leverages the existing social graph for sharing. Users can build an audience and reward followers with exclusive content. More scalable than individual sharing for influencer-style creators.

**Independent Test**: Can be tested by setting recipe visibility to followers-only and verifying only followers can access it. Delivers value for content creators with audiences.

**Acceptance Scenarios**:

1. **Given** I set a recipe to followers-only visibility, **When** a user who follows me requests it, **Then** they can view the recipe
2. **Given** a recipe is followers-only, **When** a non-follower requests it, **Then** they receive forbidden or not found
3. **Given** a user unfollows me, **When** they request my followers-only content, **Then** they can no longer access it
4. **Given** a new user starts following me, **When** they request my followers-only content, **Then** they can immediately access it

---

### User Story 6 - Share with Groups (Priority: P3)

As a user, I can create and manage groups of users. As a resource owner, I can share content with entire groups for collaborative access.

**Why this priority**: Groups provide a scalable way to manage permissions for recurring collaborations (cooking clubs, families, teams). More maintainable than individual sharing for larger circles.

**Independent Test**: Can be tested by creating a group, adding members, sharing a recipe with the group, and verifying all members can access it. Delivers value for organized sharing.

**Acceptance Scenarios**:

1. **Given** I create a group and add members, **When** I view the group details, **Then** I see all members listed
2. **Given** I own a recipe shared with a group, **When** a group member requests it, **Then** they can view the recipe
3. **Given** I remove a user from a group, **When** they request content shared with that group, **Then** they can no longer access it
4. **Given** I add a new member to a group, **When** they request content shared with that group, **Then** they can access it immediately
5. **Given** I am a group member, **When** I view my groups, **Then** I see all groups I belong to

---

### User Story 7 - Share with Federated Instances (Priority: P3)

As a resource owner, I can share content with specific federated instances, allowing users from trusted external servers to access my content.

**Why this priority**: Enables cross-instance collaboration in the federated network. Users can share with trusted communities on other servers while maintaining control. Important for federation ecosystem growth.

**Independent Test**: Can be tested by sharing content with a specific instance and verifying users from that instance can access it while users from other instances cannot. Delivers value for federated communities.

**Acceptance Scenarios**:

1. **Given** I share a recipe with a specific federated instance, **When** a user from that instance requests it, **Then** they can view the recipe
2. **Given** I share content with an instance, **When** a user from a different instance requests it, **Then** they receive forbidden or not found
3. **Given** I revoke instance sharing, **When** users from that instance request the content, **Then** they can no longer access it
4. **Given** content is shared with my instance, **When** I browse as a local user, **Then** I can discover and view that content

---

### User Story 8 - Collaborative Book Editing (Priority: P2)

As a book owner, I can grant specific users or groups the ability to add their own recipes to my book, enabling collaborative recipe collections (e.g., family cookbook, cooking club recipes) while each recipe remains owned by its original creator.

**Why this priority**: Enables family and group collaboration on shared recipe collections. This is a key differentiator for household and community use cases where multiple people contribute to a single book without transferring recipe ownership.

**Independent Test**: Can be tested by creating a book, granting a user contributor access, having them add their recipe, and verifying the recipe remains theirs while appearing in the book. Delivers value for family and group recipe sharing.

**Acceptance Scenarios**:

1. **Given** I own a book, **When** I grant a user "contributor" permission, **Then** they can add their own recipes to my book
2. **Given** I am a contributor to a book, **When** I add my recipe to the book, **Then** my recipe appears in the book but I remain the recipe owner
3. **Given** I am a contributor to a book, **When** I try to edit another contributor's recipe in the book, **Then** I am denied (recipe ownership is individual)
4. **Given** I am a contributor to a book, **When** I want to remove my recipe from the book, **Then** I can remove it (my recipe, my choice)
5. **Given** I own a book with contributors, **When** I remove a recipe added by a contributor, **Then** it is removed from the book but the recipe still exists for its owner
6. **Given** I own a book, **When** I revoke a user's contributor access, **Then** they can no longer add recipes but their previously added recipes remain in the book
7. **Given** I own a book shared with a group as contributors, **When** any group member adds their recipe, **Then** the recipe appears in the book

---

### User Story 9 - Permission Management Interface (Priority: P2)

As a resource owner, I can view and manage all sharing permissions for my content through a clear interface.

**Why this priority**: Essential for users to understand and control who has access to their content. Without this visibility, users cannot effectively manage their privacy and sharing.

**Independent Test**: Can be tested by navigating to a recipe's sharing settings and viewing/modifying permissions. Delivers value for understanding and controlling access.

**Acceptance Scenarios**:

1. **Given** I own a recipe with multiple sharing rules, **When** I view its permissions, **Then** I see all users, groups, and instances with access
2. **Given** I view my recipe's permissions, **When** I remove a permission, **Then** that entity loses access immediately
3. **Given** I want to share with a new user, **When** I add them via the interface, **Then** they gain access immediately
4. **Given** I view permissions for a recipe, **When** I check permission levels, **Then** I can distinguish between view-only and edit access

---

### Edge Cases

- What happens when a user is deleted who had shared content? (Content remains with original owner, shared-with references are cleaned up)
- How does the system handle when a recipe owner is in a group that has the recipe shared with it? (Owner permissions take precedence, no duplicate permissions)
- What happens when a group is deleted that had content shared with it? (Sharing rules for that group are removed, content returns to owner-only if no other rules exist)
- How does visibility interact with sharing rules? (Public content is accessible to all regardless of specific shares; private + shares creates a whitelist)
- What happens when a federated instance becomes unreachable? (Local permission records remain; access checks use cached instance data; periodic cleanup of stale instances)
- What happens if a user tries to share with themselves? (Ignored or prevented, owner already has full access)
- How are permission conflicts resolved when a user has access via multiple paths (direct, group, followers, instance)? (Highest permission level wins - edit > contributor > view > none)
- What happens when a contributor's recipe is deleted? (The book-recipe link is automatically removed)
- What happens when a contributor deletes their account? (Their recipes remain in the book with orphaned ownership, or are removed based on deletion policy)
- Can a contributor add someone else's recipe to a book? (No, contributors can only add recipes they own)
- What happens if the book owner adds a contributor's private recipe to the book? (Not allowed; only the recipe owner can add their recipe to books)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST enforce that only authenticated users can create resources (recipes, books)
- **FR-002**: System MUST allow resource owners full control (view, edit, delete, share) over their resources
- **FR-003**: System MUST support visibility levels: private, followers-only, public
- **FR-004**: System MUST allow sharing resources with specific users at view, edit, or contributor permission levels
- **FR-005**: System MUST allow sharing resources with groups at view, edit, or contributor permission levels
- **FR-006**: System MUST allow sharing resources with specific federated instances
- **FR-007**: System MUST evaluate permissions in order: owner check → direct share → group membership → followers → public visibility
- **FR-008**: System MUST grant the highest available permission when a user qualifies via multiple paths
- **FR-009**: System MUST propagate child resource permissions from parent (recipe images inherit from recipe)
- **FR-010**: System MUST allow users to create, edit, and delete groups they own
- **FR-011**: System MUST allow group owners to add and remove members
- **FR-012**: System MUST allow users to view groups they are members of
- **FR-013**: System MUST remove sharing rules when the target entity (user, group, instance) is deleted
- **FR-014**: System MUST log all permission changes for audit purposes
- **FR-015**: System MUST provide an interface for owners to view and manage all permissions on their resources
- **FR-016**: System MUST return 404 Not Found for all inaccessible resources to prevent information leakage about private content existence
- **FR-017**: System MUST support revoking access immediately upon permission removal
- **FR-018**: System MUST handle federated users correctly, checking their home instance for instance-level sharing
- **FR-019**: System MUST cache permission decisions for performance while ensuring cache invalidation on permission changes
- **FR-020**: System MUST prevent privilege escalation (users cannot grant permissions higher than their own access level)
- **FR-021**: System MUST grant access to all recipes in a book when the book is shared (book sharing cascades to contained recipes)
- **FR-022**: System MUST support "contributor" permission level for books, allowing users to add their own recipes to the book
- **FR-023**: System MUST preserve individual recipe ownership when recipes are added to collaborative books (contributor's recipe remains theirs)
- **FR-024**: System MUST allow recipe owners to remove their own recipes from any book they contributed to
- **FR-025**: System MUST allow book owners to remove any recipe from their book (without deleting the recipe itself)
- **FR-026**: System MUST retain contributed recipes in a book even after revoking the contributor's access

### Key Entities

- **Permission**: Represents a permanent access grant from a resource to a subject (user, group, or instance) with a level (view, edit, or contributor); no expiration support—revocation is explicit only
- **BookContribution**: Association between a book and a recipe added by a contributor, tracking who added the recipe and when
- **Group**: A named collection of users created and managed by an owner, used for batch permission grants
- **GroupMember**: Association between a user and a group they belong to
- **Visibility**: An attribute on resources indicating public access rules (private, followers-only, public)
- **FederatedInstance**: A record of known external servers in the federation, used for instance-level sharing

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Authorization decisions complete in under 50ms for 95% of requests
- **SC-002**: Users can share a resource with another user in under 30 seconds
- **SC-003**: Permission changes take effect immediately (within 1 second) for new requests
- **SC-004**: System correctly denies unauthorized access 100% of the time (no false positives)
- **SC-005**: Groups support at least 1000 members without degraded permission check performance
- **SC-006**: Users can view all permissions on a resource in a single screen/page
- **SC-007**: Permission audit logs capture all changes with actor, action, target, and timestamp
- **SC-008**: Revoking access prevents the target from accessing the resource on their next request

## Assumptions

- The existing user authentication system (session-based with AuthUser middleware) remains in place and provides a reliable authenticated user identity
- The existing follow relationship system will be leveraged for followers-only visibility
- Federated instances are identified by their domain and can be validated through ActivityPub discovery
- The existing visibility enum (Public/Private) will be extended to include FollowersOnly
- Permission checks will be integrated into the existing service layer pattern (RecipeService, BookService)
- Child resources (recipe images, ingredients, instructions) inherit permissions from their parent recipe
- The system will use a deny-by-default approach where access requires an explicit grant
- Performance requirements assume reasonable database indexing and query optimization
