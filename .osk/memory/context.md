# Contexte Projet

> Généré par `/osk-configure` le 2025-12-26
> Source de vérité factuelle du projet

## Identité

- **Nom** : Oppskrift
- **Description** : Application de partage de recettes avec fédération ActivityPub
- **Repository** : Local (branche 001-recipe-sharing)

## Stack Technique

> Détecté par `osk init` (CLI)

| Technologie | Version | Source |
|-------------|---------|--------|
| Rust | 1.75+ | Cargo.toml |
| Axum | 0.8 | Cargo.toml |
| PostgreSQL | 15+ | DATABASE_URL |
| SQLx | - | Cargo.toml |
| Docker | - | Dockerfile |
| ActivityPub | - | activitypub-federation-rust |

## Domaines Réglementaires

> Analysé par `/osk-configure` - Validé par utilisateur

| Domaine | Statut | Niveau | Justification |
|---------|--------|--------|---------------|
| RGPD | ✅ Activé | Standard | Données utilisateur (username, display_name, bio, relations, activités) |
| NIS2 | ❌ Désactivé | - | Aucun secteur réglementé détecté |
| RGS | ❌ Désactivé | - | Pas d'administration publique |

### Indices Détectés

**RGPD :**
- `src/models/user.rs:24` → `User { username, display_name, bio, avatar_url }`
- `src/api/middleware/auth.rs:13` → `Claims { sub, username }` (JWT)
- `src/models/follow.rs:6` → Relations entre utilisateurs
- `src/models/activity.rs:72` → `actor_username` traçant actions
- `src/models/saved_recipe.rs:10` → `user_id` (préférences)
- `src/models/user.rs:36` → `ap_id` (identifiant ActivityPub fédéré)

## Patterns de Sécurité Existants

| Catégorie | État | Détails |
|-----------|------|---------|
| Authentification | ✅ OK | JWT avec validation jsonwebtoken |
| Autorisation | ⚠️ Partiel | AuthUser extractor, pas de RBAC |
| Validation entrées | ✅ OK | Crate validator sur modèles |
| Logging | ⚠️ Partiel | tracing présent, pas d'audit structuré |
| Secrets | ⚠️ Attention | .env avec secrets, fallbacks dangereux |
| Escape XSS | ✅ OK | html_escape() et escape_xml() |
| SQL Injection | ✅ OK | SQLx requêtes paramétrées |

## Alertes Initiales

- [ ] JWT_SECRET fallback "dev-secret" en production (`src/api/middleware/auth.rs:87`)
- [ ] Private key placeholder pour ActivityPub (`src/jobs/federation.rs:127`)
- [ ] Implémenter audit logging structuré pour conformité RGPD
