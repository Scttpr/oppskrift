# AGENTS.md

Guidance for AI agents working in the Oppskrift codebase. Keep changes small, tested, and idiomatic to the surrounding code.

## What this is

Oppskrift is a federated recipe-sharing platform: server-rendered HTML (Askama + HTMX) over a JSON API, backed by PostgreSQL, with ActivityPub federation.

- **Language:** Rust (2021 edition)
- **Web:** Axum 0.8
- **DB:** PostgreSQL 15+ via SQLx 0.8 (compile-time-checked queries, offline mode)
- **Templates:** Askama (compiled at build time)
- **Frontend:** server-rendered HTML + HTMX, styled with Tailwind CSS v3
- **Federation:** activitypub-federation 0.6

## Project layout

```
src/
├── models/      # Data types; unit tests inline (#[cfg(test)])
├── services/    # Business logic + all SQL (query_as! etc.)
├── api/         # JSON/REST endpoints + middleware
├── handlers/    # HTML page handlers (render Askama templates)
└── core/        # Shared: config, db, error, pagination, csrf, activitypub, ...
templates/       # Askama .html (layouts/, components/, recipes/, ...)
static/          # css/ (input.css → main.css), js/ (HTMX vendored)
migrations/      # SQLx migrations (timestamped .sql)
tests/           # Integration tests; helpers in tests/common/
.sqlx/           # SQLx offline query cache (committed)
```

## Setup & running locally

A running Postgres is required to compile (SQLx macros check queries against the DB).

```bash
make db          # start postgres via docker/podman compose
make migrate     # apply migrations (sqlx migrate run)
make css         # build static/css/main.css from input.css
make dev         # build css, then `cargo watch -x run`  → http://localhost:3000
```

`.env` is loaded automatically (dotenvy); `DATABASE_URL` defaults to
`postgres://oppskrift:oppskrift@localhost:5432/oppskrift`.

## Common commands

```bash
cargo build                          # build (needs DB or SQLX_OFFLINE=true)
cargo test                           # all tests (needs DB)
cargo test --test <name>             # one integration test file
cargo clippy --all-targets -- -D warnings
cargo fmt                            # format (CI uses stable rustfmt)
SQLX_OFFLINE=true cargo build        # build against the committed .sqlx cache
```

## Database & SQLx — read before adding queries

Queries use the `sqlx::query_as!` / `query!` macros, which are verified at
compile time. When you **add or change a query**:

1. Have the dev DB running and migrations applied.
2. Regenerate the offline cache: `cargo sqlx prepare`.
3. Commit the new `.sqlx/query-*.json` file(s).

Notes:
- `cargo sqlx prepare` may rewrite an unrelated cache file due to nullability
  inference differences — revert those and commit only your new queries.
- Schema changes go in a new timestamped file in `migrations/`. Don't edit
  applied migrations. After switching branches, run `make migrate` (or
  `make reset-db` if a branch's migrations diverge).

## Testing

- Unit tests: inline `#[cfg(test)] mod tests { ... }` next to the code.
- Integration tests: `tests/*.rs`, with shared helpers in `tests/common/`
  (`run_test`, `TestContext`, `fixtures`). They hit a real DB.
- Add a test with every behavioral change; mirror the style of the nearest
  existing test file.

## Code style & conventions

- rustfmt defaults; `cargo clippy --all-targets -- -D warnings` must be clean.
- Errors: `thiserror` for libraries, `anyhow` for application glue;
  `AppError`/`AppResult` in `core::error`.
- Keep SQL in `services/`; keep handlers thin.
- Enforce access via `PermissionService` / `get_by_id_authorized` — return 404
  (not 403) for unauthorized access to private resources, to hide existence.
- Don't add comments, docstrings, or type annotations to code you didn't change.
  Match the surrounding code's idiom and density.

## CSS / theming

- Tailwind **v3** (standalone binary, pinned 3.4.17). `input.css` uses v3
  directives (`@tailwind base/components/utilities`) — not v4 syntax.
- `static/css/main.css` is **gitignored** and built by `make css` (and by the
  Dockerfile for deploys). Rebuild it after editing `input.css`,
  `tailwind.config.js`, or adding new utility classes in templates.
- Theme is driven by CSS variables (`--color-primary-*`) plus named brand
  colors in `tailwind.config.js`; reusable component classes live in
  `input.css` under `@layer components`.

## Before opening a PR

Make sure these pass (they mirror CI: fmt, clippy, test, security):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Conventions: branch off `main`, one focused change per PR, single-line
imperative commit messages, never commit secrets/`.env`. CI builds nothing CSS
related — `main.css` is produced only in the Docker image.
