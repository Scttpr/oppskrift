# French-only localization, no i18n layer

## Status

accepted

## Context

Oppskrift shipped 100% English with no internationalization infrastructure. The
goal is to make the app French. We chose to make it *French*, not *multilingual*:
replace English in place rather than build an i18n abstraction (translation keys,
locale negotiation, per-user `language` preference, English fallback). A
single-target-language project does not justify i18n scaffolding; if multilingual
support is ever needed, that is a separate, larger project and the right time to
extract.

## Decision

- **French-only, in-place.** French is written directly into templates and Rust
  string literals where they live. No central constants module, no i18n crate, no
  language switcher, no English fallback. Askama's compile-time template checking
  stays intact.
- **Scope = everything a human reads.** Web UI templates, API JSON messages,
  error/service messages that surface to users, `validator` messages,
  transactional emails (subjects + bodies), and the legal pages
  (terms/privacy/about) are all French.
- **Register: «tu».** The app addresses the user informally throughout, matching
  its community recipe-sharing tone. Applied consistently across every string.
- **Dates Frenchified, numbers not.** Dates render with French month names and
  `%-d %B %Y à %Hh%M`. Decimal numbers keep the `.` separator — switching to `,`
  risks parsing/round-trip bugs in quantity-entry forms for negligible payoff.
- **Enum display labels translated** (Difficulty → Facile/Moyen/Difficile,
  MeasurementPref → Métrique/Impérial). Rust enum *variant identifiers* and
  serialization stay English. Measurement unit abbreviations (`g`, `ml`, `tsp`…)
  are left untouched — universal or user-entered DB content.
- **`<html lang="fr">`, page titles, meta descriptions, and attribute text**
  (`placeholder`/`aria-label`/`alt`/`title`) all translated.

## Explicit no-s (stay English)

- **Wire-protocol strings** — ActivityPub vocabulary, WebFinger, oEmbed, RSS/Atom
  field names. Federation breaks if these change.
- **Logs / `tracing` output** — for operators, not users.
- **URL routes** — `/recipes`, `/books`, `/settings`. Translating them means
  rewriting every link/redirect and breaking bookmarks and federation references.
- **Reserved-username blocklist** (`models/user.rs`) — a route-collision guard
  tied to the English routes above, not UI text.

## Consequences

- Legal pages (Terms, Privacy) are translated for consistency but are functional,
  not legally vetted translations — have them reviewed before relying on them
  (note RGPD/CNIL expectations for a French-facing service).
- The DB stores user content language-agnostically; no migration of stored values.
