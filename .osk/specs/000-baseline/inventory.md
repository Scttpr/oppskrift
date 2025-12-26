# Inventaire Projet - Oppskrift

> Généré par `/osk-baseline` le 2025-12-26

## Statistiques

| Attribut | Valeur |
|----------|--------|
| Fichiers Rust | 54 |
| Lignes de code | ~7,300 |
| Templates HTML | 17 |
| Migrations SQL | 12 |

## Architecture

- **Type** : Monolithe modulaire
- **Framework** : Axum 0.8
- **Base de données** : PostgreSQL 15 (SQLx)
- **Stockage** : S3 (MinIO)
- **Fédération** : ActivityPub

## Structure

```
src/
├── api/              # REST API + ActivityPub endpoints
│   ├── middleware/   # Auth JWT, Rate limiting
│   ├── activitypub.rs
│   ├── books.rs
│   ├── feeds.rs
│   ├── recipes.rs
│   ├── social.rs
│   └── users.rs
├── handlers/         # HTML page handlers
├── jobs/             # Background jobs (federation)
├── lib/              # Shared utilities
│   ├── activitypub/  # ActivityPub types
│   ├── db.rs
│   ├── error.rs
│   ├── pagination.rs
│   └── storage.rs
├── models/           # Data models
└── services/         # Business logic

templates/            # Askama templates
migrations/           # SQLx migrations
static/               # CSS, JS
```

## Entrypoints

| Point d'entrée | Description |
|----------------|-------------|
| `src/main.rs` | Application Axum |
| `/api/v1/*` | REST API v1 |
| `/ap/*` | ActivityPub federation |
| `/.well-known/webfinger` | WebFinger discovery |
| `/feeds/*` | RSS/Atom syndication |
| `/oembed` | oEmbed endpoint |
| `/openapi.json` | OpenAPI spec |

## Dépendances Critiques

| Crate | Usage | Sécurité |
|-------|-------|----------|
| axum | Web framework | Maintenu |
| sqlx | Database | Requêtes paramétrées |
| jsonwebtoken | Auth JWT | Maintenu |
| tower-governor | Rate limiting | OK |
| validator | Input validation | OK |
| serde | Serialization | OK |
