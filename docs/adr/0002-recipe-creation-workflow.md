# End-to-end recipe creation workflow

## Status

proposed

## Context

The `/recipes/new` form was metadata-only. It captured title, description,
times, difficulty, servings, tags, and visibility, and its submit JS sent
exactly those fields as JSON. It sent **no ingredients, no instructions, and no
images** — even though `CreateRecipeRequest`, the service layer, and the schema
fully support all three. The result: creating a recipe through the UI produced a
recipe with no ingredients and no steps, i.e. not a usable recipe. Images
compound this — they upload via `POST /recipes/{id}/images`, which needs the
recipe to already exist, so a single-shot form cannot carry both text and image
bytes without a decision about ordering.

The goal is one coherent flow that takes a user from nothing to a complete,
photographed, published recipe. We chose to close the gap with the smallest
change that fits the existing server-rendered Askama + HTMX + vanilla-JS stack,
rather than a wizard or an assisted (URL/photo) ingestion feature.

## Decision

- **One comprehensive form**, not a multi-step wizard. A single page captures
  metadata + dynamic ingredient rows + step rows + one hero photo.
- **Two-phase, client-orchestrated submit.** The JS first POSTs
  recipe + ingredients + steps + tags to `/api/v1/recipes` as one atomic JSON
  request, takes the returned `id`, then uploads the hero photo to
  `POST /recipes/{id}/images`, then waits for image processing and redirects to
  the finished recipe page. Both existing endpoints are reused unchanged. Image
  upload is deliberately **not** folded into the create transaction — if the
  photo fails, the recipe still exists and the user gets a soft "retry the photo
  on the recipe page" notice rather than losing their work.
- **Ingredient row = numeric `quantity` + `unit` (free text) + `name`.** Maps
  1:1 to the model; no fraction parsing, no notes field in the create flow.
- **Step row = `description` + optional `duration` (minutes).** `duration_min`
  is already part of the `CreateInstructionStep` payload. Per-step images are
  deferred to the edit page (they would reopen the two-phase upload problem per
  step).
- **Rows are vanilla-JS template clones.** Add/remove buttons and ▲/▼ reorder
  buttons manipulate DOM nodes; `position` / `step_number` are derived from DOM
  order at submit. No HTMX round-trips (the rows must be serialized to JSON
  anyway) and no new JS framework. The form opens with one empty ingredient row
  and one empty step row.
- **Hybrid validation.** Client-side inline checks at the offending field for
  required values; blank rows are stripped; submit is blocked unless there is at
  least one real ingredient and one real step. A top summary box shows whatever
  the server still rejects.
- **The ≥1-ingredient / ≥1-step minimum is client-side only.** The
  `/api/v1/recipes` contract stays permissive. The empty-recipe problem is a UI
  problem (the old form sent nothing); the API correctly creates what it is told,
  and tightening it would be a contract change rippling into every API client.
- **`localStorage` autosave.** Form state is stashed on input, offered for
  restore on return to `/recipes/new`, and cleared on successful create.
- **Visibility defaults to Private**, aligning the form with the model default
  and the app's privacy-first stance. Public is shown with a one-line hint that
  it federates outward.
- **Single hero photo**, auto-primary, with an inline preview. Multi-image
  gallery management stays on the existing edit/recipe page.

## Consequences

- Recipe creation is **not fully atomic**: the recipe (with its ingredients and
  steps) commits atomically, but the hero image is a separate, best-effort second
  request. This is an accepted trade-off — photos are supplementary, and the
  failure mode is recoverable on the recipe page rather than data loss.
- The minimum-content rule lives only in the browser. A non-UI API client can
  still create an ingredient-less recipe; if that ever needs to be a hard product
  invariant, it must be added to `CreateRecipeRequest` validation as a separate,
  deliberate contract change.
- Changing Public to Private as the default is a behavioural change for anyone
  used to the old form; given public recipes federate over ActivityPub and are
  effectively irreversible across instances, the safer default is intentional.
- Row ordering is positional/DOM-derived, so reordering logic is presentation
  only — no stored ordering state beyond `position` / `step_number`.
