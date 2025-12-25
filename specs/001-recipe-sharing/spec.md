# Feature Specification: Recipe Creation and Sharing

**Feature Branch**: `001-recipe-sharing`
**Created**: 2025-12-25
**Status**: Draft
**Input**: User description: "User can create recipes, books recipes and share these with other users."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Create a Recipe (Priority: P1)

As a user, I want to create a recipe with all the details needed for someone else to prepare the dish, so I can share my culinary knowledge with others.

**Why this priority**: Recipe creation is the foundational action of the platform. Without recipes, there is nothing to organize into books or share. This must work first.

**Independent Test**: Can be fully tested by creating a recipe with title, ingredients, and instructions, then viewing it to confirm all entered information displays correctly.

**Acceptance Scenarios**:

1. **Given** I am logged in, **When** I create a recipe with title, description, ingredients, and step-by-step instructions, **Then** the recipe is saved and visible on my profile.
2. **Given** I am creating a recipe, **When** I add an image for the dish, **Then** the image is associated with the recipe and displayed when viewing it.
3. **Given** I have created a recipe, **When** I view the recipe, **Then** I see all details including preparation time, cooking time, servings, and difficulty level.
4. **Given** I am the author of a recipe, **When** I edit the recipe, **Then** my changes are saved and the updated version is displayed.
5. **Given** I am the author of a recipe, **When** I delete the recipe, **Then** it is removed from my profile and no longer accessible.

---

### User Story 2 - Organize Recipes into Books (Priority: P2)

As a user, I want to create recipe books (collections) to organize my recipes by theme, cuisine, or any criteria I choose, so I can keep my recipes organized and share curated collections.

**Why this priority**: Recipe books add organizational value and enable sharing of curated collections. Depends on recipes existing first.

**Independent Test**: Can be tested by creating a recipe book, adding recipes to it, and verifying the book displays the contained recipes correctly.

**Acceptance Scenarios**:

1. **Given** I am logged in, **When** I create a recipe book with a title and description, **Then** the book is saved and visible on my profile.
2. **Given** I have a recipe book, **When** I add one of my recipes to the book, **Then** the recipe appears in the book's recipe list.
3. **Given** I have a recipe book with recipes, **When** I remove a recipe from the book, **Then** the recipe is no longer in the book but still exists independently.
4. **Given** I have a recipe book, **When** I view the book, **Then** I see the book's title, description, cover image (if set), and all contained recipes.
5. **Given** I am the owner of a recipe book, **When** I delete the book, **Then** the book is removed but the recipes within it remain intact.

---

### User Story 3 - Share Recipes and Books (Priority: P3)

As a user, I want to share my recipes and recipe books with other users, so they can discover, save, and prepare my dishes.

**Why this priority**: Sharing is the social component that makes this a network rather than just a personal recipe manager. Requires recipes and books to exist first.

**Independent Test**: Can be tested by sharing a recipe, having another user view it, and confirming they can see and interact with the shared content.

**Acceptance Scenarios**:

1. **Given** I have a public recipe, **When** another user views my profile, **Then** they can see and access my public recipes.
2. **Given** I have a public recipe book, **When** another user visits the book's page, **Then** they can see the book and all public recipes within it.
3. **Given** I view someone else's recipe, **When** I choose to save it, **Then** the recipe is added to my saved recipes for quick access.
4. **Given** I view someone else's recipe, **When** I share it to my followers, **Then** the original recipe (with attribution) appears in my followers' feeds.
5. **Given** I follow another user, **When** they publish a new recipe or book, **Then** I see it in my activity feed.

---

### User Story 4 - Control Recipe Visibility (Priority: P4)

As a user, I want to control who can see my recipes and recipe books, so I can keep some content private while sharing others publicly.

**Why this priority**: Privacy controls are important but not blocking for core functionality. Users can start with public-only sharing.

**Independent Test**: Can be tested by setting a recipe to private, then confirming it is not visible to other users while remaining visible to the author.

**Acceptance Scenarios**:

1. **Given** I am creating or editing a recipe, **When** I set visibility to "private", **Then** only I can view the recipe.
2. **Given** I am creating or editing a recipe, **When** I set visibility to "public", **Then** anyone can view the recipe.
3. **Given** I have a private recipe book, **When** another user attempts to access it, **Then** they receive a message that the content is not available.
4. **Given** I change a recipe from private to public, **When** I save the change, **Then** the recipe becomes visible to others immediately.

---

### Edge Cases

- What happens when a user deletes a recipe that is in multiple recipe books? The recipe is removed from all books.
- What happens when a user tries to add someone else's recipe to their own book? They can add a reference/bookmark to the original recipe (not a copy).
- What happens when a recipe's author deletes their account? Their recipes and books are removed; saved references show "content no longer available."
- What happens when a user shares a private recipe directly with a link? Private recipes return "not found" to unauthorized users.
- What happens when viewing a recipe book where some recipes have been deleted? Deleted recipes are not shown; a placeholder may indicate "X recipes removed."
- What happens when an author edits a recipe that others have saved? Saved references always display the current (updated) version; no versioning or change notifications.

## Requirements *(mandatory)*

### Functional Requirements

**Recipe Management**
- **FR-001**: System MUST allow authenticated users to create recipes with title, description, ingredients list, and step-by-step instructions.
- **FR-002**: System MUST support adding preparation time, cooking time, servings count, and difficulty level to recipes.
- **FR-003**: System MUST allow users to upload and associate images with their recipes.
- **FR-004**: System MUST allow recipe authors to edit their own recipes.
- **FR-005**: System MUST allow recipe authors to delete their own recipes.
- **FR-006**: Recipes MUST use [Schema.org Recipe](https://schema.org/Recipe) vocabulary for data representation (per constitution).
- **FR-006a**: System MUST enforce recipe content limits: maximum 50 ingredients, 30 instruction steps, and 10 images per recipe.
- **FR-006b**: System MUST store ingredient quantities in standardized metric format and convert to user's preferred measurement system (metric or imperial) for display.

**Recipe Books**
- **FR-007**: System MUST allow authenticated users to create recipe books with title and description.
- **FR-008**: System MUST allow users to add their own recipes to their recipe books.
- **FR-009**: System MUST allow users to save references to other users' recipes in their recipe books.
- **FR-010**: System MUST allow users to remove recipes from their recipe books.
- **FR-011**: System MUST allow users to set a cover image for recipe books.
- **FR-012**: System MUST allow recipe book owners to delete their own recipe books.

**Sharing and Discovery**
- **FR-013**: System MUST display public recipes on the author's profile page.
- **FR-014**: System MUST allow users to save other users' recipes for quick access.
- **FR-015**: System MUST allow users to share recipes to their followers' activity feeds.
- **FR-016**: System MUST show new recipes and books from followed users in an activity feed.
- **FR-017**: System MUST maintain attribution when recipes are shared or referenced.

**Privacy Controls**
- **FR-018**: System MUST allow users to set recipe visibility to "public" or "private".
- **FR-019**: System MUST allow users to set recipe book visibility to "public" or "private".
- **FR-020**: System MUST prevent unauthorized users from accessing private content.
- **FR-021**: System MUST default new recipes to "public" visibility to encourage sharing and reduce friction for social features.

**Federation (per constitution)**
- **FR-022**: Recipes and recipe books MUST be representable as ActivityPub objects.
- **FR-023**: System MUST support sharing recipes across federated instances.
- **FR-024**: System MUST include proper attribution for federated content.

### Key Entities

- **Recipe**: A culinary preparation document containing title, description, ingredients, instructions, timing information, servings, difficulty, images, author reference, visibility setting, and creation/modification timestamps.

- **Ingredient**: A component of a recipe with quantity (stored in standardized metric format), unit, name, and optional preparation notes. Displayed in user's preferred measurement system (metric or imperial).

- **Instruction Step**: A single step in recipe preparation with step number, description, and optional image or timing note.

- **Recipe Book**: A user-created collection containing title, description, cover image, list of recipe references (owned or saved), owner reference, visibility setting, and creation/modification timestamps.

- **User**: A person with an account who can create content, follow others, and interact with recipes. Contains profile information, list of created recipes, list of recipe books, list of saved recipes, list of followed users, and privacy preferences.

- **Activity**: A record of actions (new recipe, new book, share) for the activity feed, containing actor, action type, target content, and timestamp.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can create a complete recipe (title, ingredients, instructions, image) in under 5 minutes.
- **SC-002**: Users can create a recipe book and add 5 recipes to it in under 2 minutes.
- **SC-003**: 90% of users successfully publish their first recipe on the first attempt.
- **SC-004**: Shared recipes display with full attribution within 3 seconds of the share action.
- **SC-005**: Activity feeds update with new content from followed users within 30 seconds.
- **SC-006**: Private content returns "not found" to unauthorized users 100% of the time (no information leakage).
- **SC-007**: System supports at least 1000 concurrent users creating and viewing recipes without degradation.
- **SC-008**: Recipe data exports include all Schema.org Recipe fields for full interoperability.

### Accessibility Outcomes (per Constitution v1.2.0)

- **SC-009**: All pages achieve WCAG 2.1 AAA compliance (AA as minimum); validated via axe-core and Lighthouse.
- **SC-010**: All user journeys completable via keyboard-only navigation with visible focus indicators.
- **SC-011**: All pages pass screen reader testing (NVDA, VoiceOver) with proper ARIA landmarks and labels.
- **SC-012**: All recipe images have meaningful alt text; form inputs have associated labels.
- **SC-013**: Color contrast meets AAA ratio (7:1 for normal text, 4.5:1 for large text).
- **SC-014**: Skip links present on all pages; high-contrast mode supported.

## Clarifications

### Session 2025-12-25

- Q: What are the maximum limits for recipe content? → A: Generous limits: 50 ingredients, 30 steps, 10 images per recipe.
- Q: Can authors edit recipes after others have saved/shared them? → A: Authors can edit freely; saved references always see the updated version.
- Q: How should ingredient quantities handle metric vs imperial? → A: Store in standardized format; display per user preference.

## Assumptions

- Users are authenticated before creating, editing, or deleting content (authentication handled by separate feature/system).
- The "follow" relationship between users exists (may be part of a separate user/social feature).
- Image storage and processing infrastructure exists or will be implemented as part of this feature.
- ActivityPub federation infrastructure exists or will be implemented as part of this feature per constitution requirements.
- State-of-the-art accessibility (WCAG 2.1 AAA target, AA minimum) applies per constitution v1.2.0.

## References

- [Schema.org Recipe](https://schema.org/Recipe) - Standard vocabulary for recipe data representation
