# Data Model: Recipe Creation and Sharing

**Feature**: 001-recipe-sharing
**Date**: 2025-12-25

## Entity Relationship Diagram

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│      User       │       │     Recipe      │       │   RecipeBook    │
├─────────────────┤       ├─────────────────┤       ├─────────────────┤
│ id (PK)         │──┐    │ id (PK)         │──┐    │ id (PK)         │
│ username        │  │    │ author_id (FK)  │◄─┤    │ owner_id (FK)   │◄─┐
│ display_name    │  │    │ title           │  │    │ title           │  │
│ bio             │  └───►│ description     │  │    │ description     │  │
│ avatar_url      │       │ visibility      │  │    │ cover_image_url │  │
│ measurement_pref│       │ prep_time_min   │  │    │ visibility      │  │
│ created_at      │       │ cook_time_min   │  │    │ created_at      │  │
│ updated_at      │       │ servings        │  │    │ updated_at      │  │
│ ap_id (unique)  │       │ difficulty      │  │    │ ap_id (unique)  │  │
└─────────────────┘       │ created_at      │  │    └─────────────────┘  │
        │                 │ updated_at      │  │            │            │
        │                 │ ap_id (unique)  │  │            │            │
        │                 └─────────────────┘  │            │            │
        │                         │            │            │            │
        │                         ▼            │            ▼            │
        │                 ┌─────────────────┐  │    ┌─────────────────┐  │
        │                 │   Ingredient    │  │    │ BookRecipeEntry │  │
        │                 ├─────────────────┤  │    ├─────────────────┤  │
        │                 │ id (PK)         │  │    │ id (PK)         │  │
        │                 │ recipe_id (FK)  │◄─┤    │ book_id (FK)    │◄─┘
        │                 │ position        │  │    │ recipe_id (FK)  │◄──┐
        │                 │ quantity        │  │    │ position        │   │
        │                 │ unit            │  │    │ added_at        │   │
        │                 │ name            │  │    └─────────────────┘   │
        │                 │ notes           │  │                          │
        │                 └─────────────────┘  │                          │
        │                         │            │                          │
        │                         ▼            │                          │
        │                 ┌─────────────────┐  │                          │
        │                 │InstructionStep  │  │                          │
        │                 ├─────────────────┤  │                          │
        │                 │ id (PK)         │  │                          │
        │                 │ recipe_id (FK)  │◄─┤                          │
        │                 │ step_number     │  │                          │
        │                 │ description     │  │                          │
        │                 │ image_url       │  │                          │
        │                 │ duration_min    │  │                          │
        │                 └─────────────────┘  │                          │
        │                         │            │                          │
        │                         ▼            │                          │
        │                 ┌─────────────────┐  │                          │
        │                 │   RecipeImage   │  │                          │
        │                 ├─────────────────┤  │                          │
        │                 │ id (PK)         │  │                          │
        │                 │ recipe_id (FK)  │◄─┘                          │
        │                 │ url             │                             │
        │                 │ alt_text        │                             │
        │                 │ position        │                             │
        │                 │ is_primary      │                             │
        │                 └─────────────────┘                             │
        │                                                                 │
        ▼                                                                 │
┌─────────────────┐       ┌─────────────────┐                             │
│   SavedRecipe   │       │    Activity     │                             │
├─────────────────┤       ├─────────────────┤                             │
│ id (PK)         │       │ id (PK)         │                             │
│ user_id (FK)    │◄──────│ actor_id (FK)   │◄────────────────────────────┘
│ recipe_id (FK)  │◄──────│ activity_type   │
│ saved_at        │       │ target_type     │
└─────────────────┘       │ target_id       │
                          │ created_at      │
┌─────────────────┐       │ ap_id (unique)  │
│     Follow      │       └─────────────────┘
├─────────────────┤
│ id (PK)         │
│ follower_id (FK)│
│ following_id(FK)│
│ created_at      │
│ ap_id (unique)  │
└─────────────────┘
```

## Entity Definitions

### User

Primary actor in the system. Represents both local and federated users.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| username | VARCHAR(50) | UNIQUE, NOT NULL | Handle (e.g., @chef) |
| display_name | VARCHAR(100) | NOT NULL | Displayed name |
| bio | TEXT | | Profile description |
| avatar_url | VARCHAR(2048) | | Profile image URL |
| measurement_pref | ENUM | DEFAULT 'metric' | 'metric' or 'imperial' |
| created_at | TIMESTAMPTZ | NOT NULL | Account creation |
| updated_at | TIMESTAMPTZ | NOT NULL | Last profile update |
| ap_id | VARCHAR(2048) | UNIQUE, NOT NULL | ActivityPub ID (URL) |

**Indexes**: username, ap_id

### Recipe

Core content entity. Maps to Schema.org Recipe.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| author_id | UUID | FK(User), NOT NULL | Recipe creator |
| title | VARCHAR(200) | NOT NULL | Recipe name |
| description | TEXT | | Brief summary |
| visibility | ENUM | DEFAULT 'public' | 'public' or 'private' |
| prep_time_min | INTEGER | CHECK >= 0 | Preparation time (minutes) |
| cook_time_min | INTEGER | CHECK >= 0 | Cooking time (minutes) |
| servings | VARCHAR(50) | | Yield (e.g., "4 servings") |
| difficulty | ENUM | | 'easy', 'medium', 'hard' |
| created_at | TIMESTAMPTZ | NOT NULL | Creation timestamp |
| updated_at | TIMESTAMPTZ | NOT NULL | Last modification |
| ap_id | VARCHAR(2048) | UNIQUE, NOT NULL | ActivityPub ID |
| search_vector | TSVECTOR | | Full-text search index |

**Indexes**: author_id, visibility, created_at DESC, ap_id, GIN(search_vector)
**Composite Index**: (visibility, created_at DESC) INCLUDE (id, title, author_id) WHERE visibility = 'public'
**Validation**: title length 1-200, max 50 ingredients, max 30 steps, max 10 images

### Ingredient

Component of a recipe. Quantities stored in metric.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| recipe_id | UUID | FK(Recipe), NOT NULL | Parent recipe |
| position | INTEGER | NOT NULL | Display order (1-based) |
| quantity | DECIMAL(10,3) | | Amount in metric units |
| unit | VARCHAR(20) | | Unit (g, ml, pieces, etc.) |
| name | VARCHAR(200) | NOT NULL | Ingredient name |
| notes | VARCHAR(200) | | Prep notes (e.g., "diced") |

**Indexes**: recipe_id, (recipe_id, position) UNIQUE
**Validation**: position 1-50

### InstructionStep

Single step in recipe preparation.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| recipe_id | UUID | FK(Recipe), NOT NULL | Parent recipe |
| step_number | INTEGER | NOT NULL | Order (1-based) |
| description | TEXT | NOT NULL | Step instructions |
| image_url | VARCHAR(2048) | | Optional step image |
| duration_min | INTEGER | CHECK >= 0 | Optional duration |

**Indexes**: recipe_id, (recipe_id, step_number) UNIQUE
**Validation**: step_number 1-30, description max 2000 chars

### RecipeImage

Images associated with a recipe.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| recipe_id | UUID | FK(Recipe), NOT NULL | Parent recipe |
| url | VARCHAR(2048) | NOT NULL | Image URL (S3 or local) |
| alt_text | VARCHAR(200) | | Accessibility text |
| position | INTEGER | NOT NULL | Display order |
| is_primary | BOOLEAN | DEFAULT false | Main recipe image |

**Indexes**: recipe_id, (recipe_id, position) UNIQUE, partial unique on (recipe_id) WHERE is_primary = true
**Validation**: position 1-10, exactly one is_primary per recipe (enforced by partial unique index)

### RecipeBook

Collection of recipes (owned or saved references).

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| owner_id | UUID | FK(User), NOT NULL | Book owner |
| title | VARCHAR(200) | NOT NULL | Book name |
| description | TEXT | | Book description |
| cover_image_url | VARCHAR(2048) | | Cover image |
| visibility | ENUM | DEFAULT 'public' | 'public' or 'private' |
| created_at | TIMESTAMPTZ | NOT NULL | Creation timestamp |
| updated_at | TIMESTAMPTZ | NOT NULL | Last modification |
| ap_id | VARCHAR(2048) | UNIQUE, NOT NULL | ActivityPub ID |

**Indexes**: owner_id, visibility, ap_id

### BookRecipeEntry

Junction table for recipes in books. Supports both owned and saved recipes.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| book_id | UUID | FK(RecipeBook), NOT NULL | Parent book |
| recipe_id | UUID | FK(Recipe), NOT NULL | Recipe reference |
| position | INTEGER | NOT NULL | Display order |
| added_at | TIMESTAMPTZ | NOT NULL | When added |

**Indexes**: book_id, recipe_id, (book_id, recipe_id) UNIQUE, (book_id, position)

### SavedRecipe

Quick-save functionality (not organized into books).

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| user_id | UUID | FK(User), NOT NULL | User who saved |
| recipe_id | UUID | FK(Recipe), NOT NULL | Saved recipe |
| saved_at | TIMESTAMPTZ | NOT NULL | When saved |

**Indexes**: user_id, recipe_id, (user_id, recipe_id) UNIQUE

### Follow

User following relationship.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| follower_id | UUID | FK(User), NOT NULL | User following |
| following_id | UUID | FK(User), NOT NULL | User being followed |
| created_at | TIMESTAMPTZ | NOT NULL | When followed |
| ap_id | VARCHAR(2048) | UNIQUE, NOT NULL | ActivityPub ID |

**Indexes**: follower_id, following_id, (follower_id, following_id) UNIQUE
**Constraints**: CHECK (follower_id != following_id) — prevent self-follow

### Activity

Activity feed entries for social features.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | PK | Unique identifier |
| actor_id | UUID | FK(User), NOT NULL | User who performed action |
| activity_type | ENUM | NOT NULL | 'create', 'share', 'follow' |
| target_type | ENUM | NOT NULL | 'recipe', 'book', 'user' |
| target_id | UUID | NOT NULL | ID of target entity |
| created_at | TIMESTAMPTZ | NOT NULL | When occurred |
| ap_id | VARCHAR(2048) | UNIQUE, NOT NULL | ActivityPub ID |

**Indexes**: actor_id, created_at DESC, (target_type, target_id), (actor_id, created_at DESC)
**Partitioning**: Consider RANGE partitioning by created_at (quarterly) for large-scale deployments

## Enums

```sql
CREATE TYPE visibility_type AS ENUM ('public', 'private');
CREATE TYPE difficulty_type AS ENUM ('easy', 'medium', 'hard');
CREATE TYPE measurement_pref AS ENUM ('metric', 'imperial');
CREATE TYPE activity_type AS ENUM ('create', 'share', 'follow');
CREATE TYPE target_type AS ENUM ('recipe', 'book', 'user');
```

## Cascade Rules

| Parent | Child | On Delete |
|--------|-------|-----------|
| User | Recipe | CASCADE (delete user's recipes) |
| User | RecipeBook | CASCADE |
| User | SavedRecipe | CASCADE |
| User | Follow | CASCADE |
| User | Activity | CASCADE |
| Recipe | Ingredient | CASCADE |
| Recipe | InstructionStep | CASCADE |
| Recipe | RecipeImage | CASCADE |
| Recipe | BookRecipeEntry | CASCADE |
| Recipe | SavedRecipe | CASCADE |
| RecipeBook | BookRecipeEntry | CASCADE |

## ActivityPub Considerations

All primary entities (User, Recipe, RecipeBook, Activity, Follow) include an `ap_id` field:
- Format: `https://{instance}/users/{username}` for users
- Format: `https://{instance}/recipes/{id}` for recipes
- Format: `https://{instance}/books/{id}` for recipe books
- Used for federation and deduplication of remote content

## Schema.org Mapping (Recipe)

| Database Field | Schema.org Property |
|----------------|---------------------|
| title | name |
| description | description |
| ingredients (join) | recipeIngredient |
| instructions (join) | recipeInstructions |
| prep_time_min | prepTime (ISO 8601 duration) |
| cook_time_min | cookTime (ISO 8601 duration) |
| servings | recipeYield |
| images (join) | image |
| author (join) | author |
| created_at | datePublished |
| updated_at | dateModified |
| difficulty | (custom extension) |

## Database Functions and Triggers

### Auto-update timestamps

```sql
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply to tables with updated_at
CREATE TRIGGER set_updated_at BEFORE UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER set_updated_at BEFORE UPDATE ON recipe
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER set_updated_at BEFORE UPDATE ON recipe_book
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();
```

### Full-text search trigger

```sql
CREATE OR REPLACE FUNCTION recipe_search_update()
RETURNS TRIGGER AS $$
BEGIN
  NEW.search_vector :=
    setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B');
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER recipe_search_trigger BEFORE INSERT OR UPDATE ON recipe
  FOR EACH ROW EXECUTE FUNCTION recipe_search_update();
```

## Materialized Views (Optional)

### User follow counts

For high-traffic deployments, cache follower/following counts:

```sql
CREATE MATERIALIZED VIEW user_follow_counts AS
SELECT
  u.id,
  COUNT(DISTINCT f1.id) FILTER (WHERE f1.following_id = u.id) AS followers_count,
  COUNT(DISTINCT f2.id) FILTER (WHERE f2.follower_id = u.id) AS following_count
FROM users u
LEFT JOIN follow f1 ON f1.following_id = u.id
LEFT JOIN follow f2 ON f2.follower_id = u.id
GROUP BY u.id;

CREATE UNIQUE INDEX ON user_follow_counts(id);

-- Refresh periodically or via application trigger
-- REFRESH MATERIALIZED VIEW CONCURRENTLY user_follow_counts;
```

## Polymorphic Association Note

The `Activity` table uses a polymorphic pattern (`target_type` + `target_id`) which cannot have foreign key constraints. Referential integrity must be enforced at the application level:

- When deleting a Recipe/Book/User, also delete related Activity rows
- Consider adding nullable typed columns if strict FK enforcement is required:
  ```sql
  -- Alternative: explicit FK columns
  target_recipe_id UUID REFERENCES recipe(id) ON DELETE CASCADE,
  target_book_id UUID REFERENCES recipe_book(id) ON DELETE CASCADE,
  target_user_id UUID REFERENCES users(id) ON DELETE CASCADE,
  CHECK (
    (target_recipe_id IS NOT NULL)::int +
    (target_book_id IS NOT NULL)::int +
    (target_user_id IS NOT NULL)::int = 1
  )
  ```
