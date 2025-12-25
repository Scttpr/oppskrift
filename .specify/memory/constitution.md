<!--
Sync Impact Report
==================
Version change: 1.1.0 → 1.2.0

Modified principles:
- IV. User Experience: Strengthened accessibility requirements to "state-of-the-art"

Added requirements:
- WCAG 2.1 AAA as target (AA minimum)
- Automated + manual accessibility testing required
- Visible focus indicators (3:1 contrast)
- Alt text requirements for all images
- Form label and error association
- Color not sole information carrier
- High contrast user preference
- Skip links for keyboard navigation
- ARIA landmarks for page regions

Removed sections: None

Templates requiring updates:
- specs/*/spec.md: Update success criteria for AAA target
- specs/*/tasks.md: Add accessibility validation tasks
- specs/*/plan.md: Note accessibility requirements in constitution check

Follow-up TODOs:
- Update 001-recipe-sharing spec, plan, and tasks for new accessibility standards
-->

# Oppskrift Constitution

## Core Principles

### I. Federation First

Oppskrift MUST implement ActivityPub as the primary federation protocol for all social features.

- Every shareable entity (recipes, collections, reviews, user profiles) MUST be representable as ActivityPub objects
- The ActivityPub implementation MUST conform to the W3C ActivityPub specification
- Federation MUST be enabled by default; instance administrators MAY configure federation policies
- All federated content MUST include proper attribution and source references
- The system MUST handle federation failures gracefully with retry mechanisms and clear error states

**Rationale**: Federation enables users to interact across instances, prevents vendor lock-in, and ensures the network effect benefits all participants rather than a single operator.

### II. Security by Default

All features MUST be secure without requiring additional configuration or opt-in from users or administrators.

- Authentication MUST use modern, proven standards (OAuth 2.0, OIDC, or equivalent)
- All network communication MUST use TLS 1.3 or higher; HTTP MUST redirect to HTTPS
- User data MUST be encrypted at rest using AES-256 or equivalent
- Input validation MUST occur at system boundaries; output encoding MUST prevent injection attacks
- Rate limiting MUST be enabled by default on all public endpoints
- Secrets, tokens, and credentials MUST never appear in logs, error messages, or client responses
- Dependencies MUST be regularly audited for known vulnerabilities

**Rationale**: Users trust the platform with personal data and social connections. Security failures damage that trust irreparably and expose users to harm.

### III. Standards Compliance

Oppskrift MUST prioritize established standards over custom implementations.

- Recipe data MUST use Schema.org Recipe vocabulary for interoperability
- API responses MUST conform to JSON-LD and Activity Streams 2.0 where applicable
- HTTP APIs MUST follow REST principles with appropriate status codes and methods
- Date/time values MUST use ISO 8601 format; all times MUST include timezone information
- The system MUST support WebFinger for user discovery across federated instances
- Content negotiation MUST be supported for serving both human-readable and machine-readable formats

**Rationale**: Standards compliance reduces integration friction, enables third-party tooling, and ensures longevity as the ecosystem evolves.

### IV. User Experience

The interface MUST be visually appealing while remaining accessible and intuitive. Accessibility MUST be state-of-the-art, not an afterthought.

- The UI MUST achieve WCAG 2.1 AAA compliance as the target; AA is the minimum acceptable
- Accessibility MUST be validated with automated tools (axe, Lighthouse) AND manual testing (screen readers, keyboard-only navigation)
- All interactive elements MUST have visible focus indicators meeting 3:1 contrast ratio
- All images MUST have meaningful alt text; decorative images MUST be marked as such
- Form inputs MUST have associated labels; error messages MUST be programmatically linked to fields
- Color MUST NOT be the sole means of conveying information
- Core user journeys (browse, search, view recipe, follow user) MUST be completable in 3 clicks or fewer
- The interface MUST be fully functional without JavaScript for critical paths
- Page load time MUST be under 3 seconds on a 3G connection for above-the-fold content
- Mobile-first responsive design MUST be employed; the interface MUST be fully usable on viewports from 320px
- Error states MUST provide actionable guidance, not technical jargon
- The system MUST respect user preferences (dark mode, reduced motion, language, high contrast)
- Skip links MUST be provided for keyboard navigation
- ARIA landmarks MUST be used to define page regions

**Rationale**: An appealing interface that frustrates users fails its purpose. Accessibility is not optional—it expands reach and improves experience for everyone. State-of-the-art accessibility demonstrates respect for all users and sets the standard for the ecosystem.

### V. Maintainability

The codebase and infrastructure MUST be easy to understand, modify, and deploy.

- Single-command deployment MUST be possible using containers (Docker, Podman, or equivalent)
- Configuration MUST use environment variables with sensible, documented defaults
- The system MUST be deployable on modest hardware (2 vCPU, 2GB RAM for small instances)
- Code MUST be organized into clear modules with explicit boundaries and dependencies
- External dependencies MUST be minimized; each dependency MUST justify its inclusion
- Database migrations MUST be versioned, reversible, and automated
- Comprehensive logging MUST enable debugging without access to production systems

**Rationale**: Open source projects live or die by contributor experience. Complex deployment or opaque code discourages participation and forks.

### VI. Open Source Ethos

Oppskrift MUST embody open source values in code, community, and governance.

- All source code MUST be licensed under an OSI-approved open source license
- Documentation MUST be sufficient for users to self-host without contacting maintainers
- Contributions MUST be welcomed; a clear contribution guide MUST exist
- Decision-making processes MUST be transparent and documented
- The project MUST NOT require proprietary services or software for core functionality
- Telemetry, if any, MUST be opt-in with clear disclosure of data collected

**Rationale**: Open source is more than a license—it is a commitment to transparency, collaboration, and user empowerment.

## Technical Constraints

### Technology Requirements

- **Database**: MUST support PostgreSQL as the primary database; other databases MAY be supported
- **Search**: Full-text search MUST be available for recipes; external search services (Elasticsearch, Meilisearch) SHOULD be optional
- **Media**: Recipe images MUST be stored with configurable backends (local filesystem, S3-compatible, or equivalent)
- **Background Jobs**: Asynchronous tasks (federation, email, media processing) MUST use a reliable job queue
- **Caching**: Response caching MUST be configurable; Redis or equivalent MAY be used but MUST NOT be required for basic operation

### Interoperability Requirements

- **Import/Export**: Users MUST be able to export their data in standard formats (JSON-LD, Atom)
- **API**: A public, documented API MUST exist for third-party integrations
- **Embedding**: Recipes MUST be embeddable via oEmbed or equivalent standards
- **RSS/Atom**: Public content MUST be available via RSS or Atom feeds

## Development Workflow

### Code Quality Gates

- All code changes MUST pass automated tests before merge
- Code review MUST be required for changes to security-sensitive components
- Linting and formatting MUST be enforced via CI; style decisions MUST NOT be manual review items
- Test coverage MUST be maintained or improved; coverage regressions MUST be justified

### Commit Conventions

All commits MUST follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
type(scope): description
```

**Required types**:
- `feat`: New feature or capability
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style (formatting, no logic change)
- `refactor`: Code restructuring (no feature or fix)
- `test`: Adding or updating tests
- `chore`: Build, tooling, dependencies, config
- `perf`: Performance improvement
- `ci`: CI/CD configuration

**Scope** SHOULD match the task category or module (e.g., `recipe`, `auth`, `feed`, `api`, `db`).

**Rules**:
- Each task from tasks.md MUST result in exactly one commit
- Commit message MUST be a single line, max 72 characters
- Description MUST be lowercase, imperative mood ("add" not "added" or "adds")
- Breaking changes MUST include `!` after type/scope (e.g., `feat(api)!: change response format`)

**Examples**:
```
feat(recipe): add ingredient validation (max 50)
fix(auth): prevent self-follow in database constraint
chore(db): create user table migration
test(recipe): add seed data for limit testing
docs(api): add openapi documentation generation
```

### Documentation Requirements

- API endpoints MUST be documented with request/response examples
- Configuration options MUST be documented with types, defaults, and effects
- Architecture decisions MUST be recorded in ADRs (Architecture Decision Records)
- User-facing changes MUST include changelog entries

### Release Process

- Releases MUST follow semantic versioning (MAJOR.MINOR.PATCH)
- Breaking changes MUST be documented and migration paths provided
- Security releases MUST be expedited and clearly communicated

## Governance

This constitution establishes the non-negotiable principles for Oppskrift development. All contributions, architectural decisions, and feature implementations MUST align with these principles.

### Amendment Process

1. Proposed amendments MUST be documented with rationale and impact analysis
2. Amendments MUST be reviewed by active maintainers
3. Breaking amendments (principle removal or redefinition) require migration plans
4. All amendments MUST update the version number according to semantic versioning

### Compliance

- Pull requests MUST include a constitution compliance check for significant changes
- Architecture Decision Records MUST reference relevant constitutional principles
- Complexity beyond what principles require MUST be justified in writing

### Versioning Policy

- **MAJOR**: Principle removal, redefinition, or backward-incompatible governance changes
- **MINOR**: New principle added, material expansion of existing guidance
- **PATCH**: Clarifications, wording improvements, typo fixes

**Version**: 1.2.0 | **Ratified**: 2025-12-25 | **Last Amended**: 2025-12-25
