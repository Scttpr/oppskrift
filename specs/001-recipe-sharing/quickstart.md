# Quickstart: Recipe Creation and Sharing

**Feature**: 001-recipe-sharing
**Date**: 2025-12-25

This guide walks through validating the Recipe Creation and Sharing feature from a user's perspective.

## Prerequisites

- Running Oppskrift instance (local or deployed)
- Two test user accounts (for social features)
- Browser with developer tools

## Test Scenarios

### 1. Create a Recipe (US1 - P1)

**Goal**: Verify basic recipe creation flow

1. **Login** as test user
2. **Navigate** to "Create Recipe" (should be accessible from main navigation)
3. **Fill form**:
   - Title: "Classic Chocolate Chip Cookies"
   - Description: "Soft and chewy cookies with melted chocolate chips"
   - Prep time: 15 minutes
   - Cook time: 12 minutes
   - Servings: "24 cookies"
   - Difficulty: Medium
4. **Add ingredients**:
   ```
   - 225g butter, softened
   - 200g brown sugar
   - 100g white sugar
   - 2 eggs
   - 5ml vanilla extract
   - 280g all-purpose flour
   - 5g baking soda
   - 3g salt
   - 340g chocolate chips
   ```
5. **Add instructions**:
   ```
   1. Preheat oven to 190°C (375°F)
   2. Cream butter and sugars until fluffy (3 minutes)
   3. Beat in eggs and vanilla
   4. Mix flour, baking soda, and salt in separate bowl
   5. Gradually add dry ingredients to wet
   6. Fold in chocolate chips
   7. Drop rounded tablespoons onto baking sheet
   8. Bake 10-12 minutes until golden edges
   9. Cool on pan 5 minutes, then transfer to rack
   ```
6. **Upload image** (optional)
7. **Save** recipe

**Expected**:
- Recipe appears on profile
- All fields display correctly
- Total time shows 27 minutes
- Visibility is "public" by default

**Verify API** (Developer Tools → Network):
- POST `/api/v1/recipes` returns 201
- Response includes Schema.org fields
- `ap_id` is generated

### 2. Edit Recipe

1. **View** the created recipe
2. **Click** "Edit"
3. **Modify** description: add "Perfect for holidays!"
4. **Save**

**Expected**:
- Changes saved immediately
- `updated_at` timestamp changes
- No new recipe created (same ID)

### 3. Create Recipe Book (US2 - P2)

1. **Navigate** to "My Books" or "Create Book"
2. **Create** new book:
   - Title: "Holiday Baking"
   - Description: "Festive treats for the season"
3. **Add** the chocolate chip cookie recipe
4. **View** book

**Expected**:
- Book appears on profile
- Cookie recipe listed in book
- Recipe count shows 1

### 4. Share and Discover (US3 - P3)

**Setup**: Login as second test user

1. **Visit** first user's profile
2. **View** their public recipes
3. **Save** the cookie recipe (bookmark icon)
4. **Follow** the first user
5. **Check** activity feed

**Expected**:
- Public recipes visible
- Recipe appears in "Saved" list
- Activity feed shows follow action

**Back to first user**:
1. **Share** the cookie recipe
2. **Check** that second user sees it in feed

**Expected**:
- Shared recipe appears in follower's feed
- Attribution preserved

### 5. Privacy Controls (US4 - P4)

1. **Create** new recipe with visibility "private"
2. **Verify** it's visible on own profile
3. **As second user**, try to access private recipe URL

**Expected**:
- Private recipe: 404 "Not found" (no information leakage)
- Private recipe NOT in public profile list

### 6. Metric/Imperial Conversion

1. **As first user**, set measurement preference to "imperial"
2. **View** cookie recipe

**Expected**:
- Quantities display in imperial (cups, oz, tsp)
- e.g., "225g butter" → "1 cup butter"

3. **Switch** to metric
4. **Verify** original metric values display

### 7. Content Limits Validation

1. **Try** creating recipe with 51 ingredients

**Expected**:
- Validation error before save
- Clear error message about limit

### 8. Schema.org Output

1. **Request** recipe with `Accept: application/ld+json`
2. **Verify** JSON-LD structure

```bash
curl -H "Accept: application/ld+json" \
  https://instance/api/v1/recipes/{id}
```

**Expected**:
```json
{
  "@context": "https://schema.org",
  "@type": "Recipe",
  "name": "Classic Chocolate Chip Cookies",
  "recipeIngredient": ["225g butter, softened", ...],
  "recipeInstructions": [
    {"@type": "HowToStep", "text": "Preheat oven..."}
  ],
  ...
}
```

## Performance Validation

| Test | Method | Target |
|------|--------|--------|
| Page load (3G) | Chrome DevTools → Throttling | <3 seconds |
| Create recipe API | Network timing | <200ms p95 |
| Feed refresh | Time from click to update | <30 seconds |

## Accessibility Validation

1. **Run** Lighthouse accessibility audit
2. **Target**: Score ≥90 (WCAG 2.1 AA)
3. **Test** keyboard navigation through recipe creation
4. **Verify** image alt text prompts

## Federation Validation (if enabled)

1. **Search** for user from another instance: `@user@other.instance`
2. **Follow** federated user
3. **Verify** their public recipes appear in feed

## Cleanup

1. Delete test recipes
2. Delete test books
3. Unfollow test users

## Common Issues

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| Images not uploading | S3 config | Check storage backend settings |
| Feed not updating | Job queue | Verify background worker running |
| 500 on create | Validation | Check server logs for constraint |
| Slow page loads | Missing indexes | Run `EXPLAIN ANALYZE` on queries |
