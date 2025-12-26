# Taches d'Implementation - Baseline

> Genere par `/osk-tasks baseline` le 2025-12-26
> Source: plan.md (19 actions -> 42 taches)

## Resume

| Phase | Taches | Effort | Parallelisables |
|-------|--------|--------|-----------------|
| 1. Fondations | 7 | 1h35 | 2 |
| 2. Core Securite | 16 | 11h45 | 6 |
| 3. Hardening | 13 | 17h | 4 |
| 4. Conformite | 6 | 16h | 2 |
| **Total** | **42** | **~46h** | **14** |

---

## Graphe de Dependances

```
PHASE 1 - FONDATIONS (P0)
========================
T001 ─────────────────────────────────────────────────────────────────> DONE
  │
T002 ──> T003 ──> T004 ──> T005 ──────────────────────────────────────> DONE
  │
T006 ──> T007 ────────────────────────────────────────────────────────> DONE

PHASE 2 - CORE SECURITE (P1)
============================
T101 ──> T102 ──> T103 ─────────────────────────────> T301 (Phase 3)
  │
T104 ──────────────────────────────────────────────────────────────────> DONE
  │
T105 ──> T106 ──> T107 ──> T108 ────────────────────> T302 (Phase 3)
  │
T109 ──────────────────────────────────────────────────────────────────> DONE
  │
T110 ──> T111 ─────────────────────────────────────────────────────────> DONE
  │
T112 ──────────────────────────────────────────────────────────────────> DONE

PHASE 3 - HARDENING (P2)
========================
T301 ──> T302 ──> T303 ─────────────────────────────────────────────────> DONE
  │
T304 ──> T305 ──> T306 ──> T307 ──> T308 ───────────────────────────────> DONE
  │
T309 ──> T310 ─────────────────────────────────────────────────────────> DONE
  │
T311 ──> T312 ──> T313 ─────────────────────────────────────────────────> DONE

PHASE 4 - CONFORMITE (P3)
=========================
T401 ──> T402 ──> T403 ─────────────────────────────────────────────────> DONE
  │
T404 ──> T405 ─────────────────────────────────────────────────────────> DONE
  │
T406 ──────────────────────────────────────────────────────────────────> DONE
```

---

## Phase 1 : Fondations

### T001 - Corriger JWT_SECRET fallback

| Attribut | Valeur |
|----------|--------|
| ID | T001 |
| Action | FIX-001 |
| Type | code |
| Priorite | P0 |
| Effort | XS (5min) |
| Depends | - |
| Blocks | - |
| Risque | VULN-BASELINE-001 |

**Instructions :**
1. Ouvrir `src/api/middleware/auth.rs`
2. Ligne 87: remplacer `unwrap_or_else(|_| "dev-secret".to_string())` par `expect("JWT_SECRET must be set")`
3. Sauvegarder

**Code :**
```rust
// Ligne 87 - AVANT
let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret".to_string());

// Ligne 87 - APRES
let secret = std::env::var("JWT_SECRET")
    .expect("JWT_SECRET environment variable must be set");
```

**Done Criteria :**
- [ ] Code modifie
- [ ] `cargo check` passe
- [ ] App refuse de demarrer sans JWT_SECRET

---

### T002 - Creer structure Config

| Attribut | Valeur |
|----------|--------|
| ID | T002 |
| Action | FIX-002 |
| Type | code |
| Priorite | P0 |
| Effort | S (30min) |
| Depends | - |
| Blocks | T003, T004 |

**Instructions :**
1. Creer fichier `src/lib/config.rs`
2. Definir struct `Config` avec champs requis
3. Implementer `Config::from_env()`

**Code :**
```rust
//! src/lib/config.rs
use std::env;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub base_url: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_endpoint: Option<String>,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let jwt_secret = env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set");
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters");
        }
        let s3_bucket = env::var("S3_BUCKET")
            .expect("S3_BUCKET must be set");
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        let s3_region = env::var("S3_REGION")
            .unwrap_or_else(|_| "us-east-1".to_string());
        let s3_endpoint = env::var("S3_ENDPOINT").ok();
        let host = env::var("HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .expect("PORT must be a valid number");
        Self {
            database_url, jwt_secret, base_url, s3_bucket,
            s3_region, s3_endpoint, host, port,
        }
    }
}
```

**Done Criteria :**
- [ ] Fichier cree
- [ ] `cargo check` passe

---

### T003 - Exporter Config dans lib.rs

| Attribut | Valeur |
|----------|--------|
| ID | T003 |
| Action | FIX-002 |
| Type | code |
| Priorite | P0 |
| Effort | XS (2min) |
| Depends | T002 |
| Blocks | T004 |

**Instructions :**
1. Ouvrir `src/lib/mod.rs`
2. Ajouter `pub mod config;`

**Done Criteria :**
- [ ] Module exporte
- [ ] `cargo check` passe

---

### T004 - Integrer Config dans main.rs

| Attribut | Valeur |
|----------|--------|
| ID | T004 |
| Action | FIX-003 |
| Type | code |
| Priorite | P0 |
| Effort | XS (15min) |
| Depends | T003 |
| Blocks | T005 |

**Instructions :**
1. Ouvrir `src/main.rs`
2. Importer Config
3. Appeler `Config::from_env()` au demarrage
4. Utiliser config pour host/port

**Code :**
```rust
use crate::lib::config::Config;

#[tokio::main]
async fn main() {
    // Load and validate config first
    let config = Config::from_env();

    // ... existing code ...

    let addr = format!("{}:{}", config.host, config.port);
    // ...
}
```

**Done Criteria :**
- [ ] Config charge au demarrage
- [ ] App utilise config.host/port
- [ ] `cargo check` passe

---

### T005 - Tester validation secrets

| Attribut | Valeur |
|----------|--------|
| ID | T005 |
| Action | FIX-003 |
| Type | test |
| Priorite | P0 |
| Effort | XS (10min) |
| Depends | T004 |
| Blocks | - |

**Instructions :**
1. Tester demarrage sans JWT_SECRET -> doit panic
2. Tester demarrage avec JWT_SECRET < 32 chars -> doit panic
3. Tester demarrage avec JWT_SECRET valide -> doit fonctionner

**Done Criteria :**
- [ ] Panic si JWT_SECRET manquant
- [ ] Panic si JWT_SECRET trop court
- [ ] Demarrage OK si valide

---

### T006 - Mettre a jour .env.example

| Attribut | Valeur |
|----------|--------|
| ID | T006 |
| Action | FIX-002 |
| Type | doc |
| Priorite | P0 |
| Effort | XS (5min) |
| Depends | T002 |
| Blocks | T007 |

**Instructions :**
1. Ouvrir `.env.example`
2. Ajouter commentaires sur secrets requis
3. Indiquer longueur minimale JWT_SECRET

**Done Criteria :**
- [ ] .env.example documente secrets requis

---

### T007 - Verifier cargo test

| Attribut | Valeur |
|----------|--------|
| ID | T007 |
| Action | FIX-003 |
| Type | test |
| Priorite | P0 |
| Effort | XS (5min) |
| Depends | T005, T006 |
| Blocks | Phase 2 |

**Instructions :**
1. Executer `cargo test`
2. Verifier 0 echec

**Done Criteria :**
- [ ] `cargo test` passe
- [ ] Phase 1 complete

---

## Phase 2 : Core Securite

### T101 - Creer structure AuditEvent

| Attribut | Valeur |
|----------|--------|
| ID | T101 |
| Action | FIX-004 |
| Type | code |
| Priorite | P1 |
| Effort | S (45min) |
| Depends | Phase 1 |
| Blocks | T102 |

**Instructions :**
1. Creer `src/lib/audit.rs`
2. Definir struct AuditEvent avec champs: timestamp, event, level, trace_id, user_id, ip, target_type, target_id, metadata

**Done Criteria :**
- [ ] Struct definie avec Serialize
- [ ] Builders with_user, with_ip, with_target

---

### T102 - Implementer AuditEvent::log()

| Attribut | Valeur |
|----------|--------|
| ID | T102 |
| Action | FIX-004 |
| Type | code |
| Priorite | P1 |
| Effort | XS (15min) |
| Depends | T101 |
| Blocks | T103, T301 |

**Instructions :**
1. Implementer methode `log()` sur AuditEvent
2. Serialiser en JSON et logger via tracing

**Done Criteria :**
- [ ] Log JSON structure
- [ ] Niveaux info/warn/error

---

### T103 - Exporter audit dans lib.rs

| Attribut | Valeur |
|----------|--------|
| ID | T103 |
| Action | FIX-004 |
| Type | code |
| Priorite | P1 |
| Effort | XS (2min) |
| Depends | T102 |
| Blocks | T301 |

**Instructions :**
1. Ajouter `pub mod audit;` dans `src/lib/mod.rs`

**Done Criteria :**
- [ ] Module exporte

---

### T104 - Creer .pre-commit-config.yaml

| Attribut | Valeur |
|----------|--------|
| ID | T104 |
| Action | FIX-005 |
| Type | config |
| Priorite | P1 |
| Effort | XS (15min) |
| Depends | - |
| Blocks | - |
| Parallele | Oui |

**Instructions :**
1. Creer `.pre-commit-config.yaml`
2. Configurer hook gitleaks

**Code :**
```yaml
repos:
  - repo: https://github.com/gitleaks/gitleaks
    rev: v8.18.0
    hooks:
      - id: gitleaks
        name: Detect hardcoded secrets
        entry: gitleaks protect --staged --verbose
```

**Done Criteria :**
- [ ] Fichier cree
- [ ] `pre-commit install` fonctionne

---

### T105 - Creer migration user_keys

| Attribut | Valeur |
|----------|--------|
| ID | T105 |
| Action | FIX-006 |
| Type | code |
| Priorite | P1 |
| Effort | S (30min) |
| Depends | - |
| Blocks | T106 |
| Parallele | Oui |

**Instructions :**
1. Creer migration SQL pour table user_keys
2. Champs: id, user_id, public_key_pem, private_key_pem_encrypted, algorithm, created_at, rotated_at

**Done Criteria :**
- [ ] Migration creee
- [ ] `sqlx migrate run` passe

---

### T106 - Ajouter dependance rsa

| Attribut | Valeur |
|----------|--------|
| ID | T106 |
| Action | FIX-007 |
| Type | setup |
| Priorite | P1 |
| Effort | XS (5min) |
| Depends | T105 |
| Blocks | T107 |

**Instructions :**
1. Ajouter `rsa = "0.9"` dans Cargo.toml
2. Ajouter `rand = "0.8"` si pas present

**Done Criteria :**
- [ ] Dependencies ajoutees
- [ ] `cargo check` passe

---

### T107 - Implementer generation RSA

| Attribut | Valeur |
|----------|--------|
| ID | T107 |
| Action | FIX-007 |
| Type | code |
| Priorite | P1 |
| Effort | S (1h) |
| Depends | T106 |
| Blocks | T108 |

**Instructions :**
1. Creer fonction `generate_rsa_keypair()` dans `src/lib/crypto.rs`
2. Generer paire RSA 2048 bits
3. Retourner (public_pem, private_pem)

**Done Criteria :**
- [ ] Fonction cree
- [ ] Cles valides generees

---

### T108 - Integrer RSA dans UserService::create

| Attribut | Valeur |
|----------|--------|
| ID | T108 |
| Action | FIX-007 |
| Type | code |
| Priorite | P1 |
| Effort | S (45min) |
| Depends | T107 |
| Blocks | T302 |

**Instructions :**
1. Modifier `UserService::create()`
2. Generer paire RSA a la creation
3. Inserer dans table user_keys

**Done Criteria :**
- [ ] Chaque nouveau user a une paire RSA
- [ ] Tests passent

---

### T109 - Creer .github/dependabot.yml

| Attribut | Valeur |
|----------|--------|
| ID | T109 |
| Action | FIX-008 |
| Type | config |
| Priorite | P1 |
| Effort | XS (15min) |
| Depends | - |
| Blocks | - |
| Parallele | Oui |

**Instructions :**
1. Creer `.github/dependabot.yml`
2. Configurer pour Cargo et GitHub Actions

**Done Criteria :**
- [ ] Fichier cree
- [ ] Dependabot activera PRs auto

---

### T110 - Rediger politique confidentialite

| Attribut | Valeur |
|----------|--------|
| ID | T110 |
| Action | RGPD-001 |
| Type | doc |
| Priorite | P1 |
| Effort | M (3h) |
| Depends | - |
| Blocks | T111 |
| Parallele | Oui |

**Instructions :**
1. Creer `docs/legal/privacy-policy.md`
2. Inclure: donnees collectees, finalites, bases legales, durees, droits

**Done Criteria :**
- [ ] Document complet conforme Art. 13-14

---

### T111 - Publier politique sur site

| Attribut | Valeur |
|----------|--------|
| ID | T111 |
| Action | RGPD-001 |
| Type | code |
| Priorite | P1 |
| Effort | S (30min) |
| Depends | T110 |
| Blocks | - |

**Instructions :**
1. Creer route `/privacy`
2. Afficher politique de confidentialite

**Done Criteria :**
- [ ] Page accessible publiquement

---

### T112 - Documenter bases legales

| Attribut | Valeur |
|----------|--------|
| ID | T112 |
| Action | RGPD-002 |
| Type | doc |
| Priorite | P1 |
| Effort | S (2h) |
| Depends | - |
| Blocks | - |
| Parallele | Oui |

**Instructions :**
1. Creer `docs/legal/legal-basis.md`
2. Lister chaque traitement et sa base legale

**Done Criteria :**
- [ ] Document complet

---

## Phase 3 : Hardening

### T301 - Configurer JSON logging production

| Attribut | Valeur |
|----------|--------|
| ID | T301 |
| Action | FIX-009 |
| Type | code |
| Priorite | P2 |
| Effort | S (1h) |
| Depends | T103 |
| Blocks | T302 |

**Instructions :**
1. Modifier setup tracing dans main.rs
2. Si RUST_ENV=production, utiliser format JSON

**Done Criteria :**
- [ ] Logs JSON en production
- [ ] Logs texte en dev

---

### T302 - Ajouter audit auth.login.success

| Attribut | Valeur |
|----------|--------|
| ID | T302 |
| Action | FIX-010 |
| Type | code |
| Priorite | P2 |
| Effort | XS (15min) |
| Depends | T301, T108 |
| Blocks | T303 |

**Instructions :**
1. Dans handler login, ajouter AuditEvent::new("auth.login.success")
2. Inclure user_id et IP

**Done Criteria :**
- [ ] Event logue a chaque login reussi

---

### T303 - Ajouter autres audit events

| Attribut | Valeur |
|----------|--------|
| ID | T303 |
| Action | FIX-010 |
| Type | code |
| Priorite | P2 |
| Effort | M (3h) |
| Depends | T302 |
| Blocks | - |

**Instructions :**
Ajouter events pour:
- auth.login.failure
- auth.token.invalid
- user.create, user.update, user.delete
- recipe.create, recipe.update, recipe.delete
- federation.inbox.received, federation.inbox.rejected

**Done Criteria :**
- [ ] 14 events implementes

---

### T304 - Implementer fetch_actor

| Attribut | Valeur |
|----------|--------|
| ID | T304 |
| Action | FIX-012 |
| Type | code |
| Priorite | P2 |
| Effort | S (1h) |
| Depends | T108 |
| Blocks | T305 |

**Instructions :**
1. Creer fonction fetch_actor(key_id) dans activitypub
2. Fetch Actor JSON depuis key_id URL
3. Extraire publicKey

**Done Criteria :**
- [ ] Peut recuperer cle publique distante

---

### T305 - Implementer verify_signature

| Attribut | Valeur |
|----------|--------|
| ID | T305 |
| Action | FIX-012 |
| Type | code |
| Priorite | P2 |
| Effort | M (3h) |
| Depends | T304 |
| Blocks | T306 |

**Instructions :**
1. Parser header Signature
2. Reconstruire signing string
3. Verifier avec RSA-SHA256

**Done Criteria :**
- [ ] Verification signatures fonctionnelle

---

### T306 - Integrer verification dans user_inbox

| Attribut | Valeur |
|----------|--------|
| ID | T306 |
| Action | FIX-012 |
| Type | code |
| Priorite | P2 |
| Effort | S (1h) |
| Depends | T305 |
| Blocks | T307 |

**Instructions :**
1. Appeler verify_signature dans handler user_inbox
2. Rejeter 401 si invalide

**Done Criteria :**
- [ ] Inbox user verifie signatures

---

### T307 - Integrer verification dans shared_inbox

| Attribut | Valeur |
|----------|--------|
| ID | T307 |
| Action | FIX-012 |
| Type | code |
| Priorite | P2 |
| Effort | XS (30min) |
| Depends | T306 |
| Blocks | T308 |

**Instructions :**
1. Appeler verify_signature dans handler shared_inbox
2. Rejeter 401 si invalide

**Done Criteria :**
- [ ] Shared inbox verifie signatures

---

### T308 - Tests verification signatures

| Attribut | Valeur |
|----------|--------|
| ID | T308 |
| Action | FIX-012 |
| Type | test |
| Priorite | P2 |
| Effort | S (1h30) |
| Depends | T307 |
| Blocks | - |

**Instructions :**
1. Creer tests unitaires pour parsing signature
2. Creer tests integration avec vraies signatures

**Done Criteria :**
- [ ] Tests couvrent cas valide et invalide

---

### T309 - Creer endpoint export donnees

| Attribut | Valeur |
|----------|--------|
| ID | T309 |
| Action | RGPD-004 |
| Type | code |
| Priorite | P2 |
| Effort | M (3h) |
| Depends | - |
| Blocks | T310 |
| Parallele | Oui |

**Instructions :**
1. Creer `GET /api/v1/users/me/export`
2. Collecter toutes donnees utilisateur
3. Retourner JSON structure

**Done Criteria :**
- [ ] Endpoint fonctionnel
- [ ] Inclut: profil, recettes, follows, likes, saved

---

### T310 - Ajouter format ActivityPub export

| Attribut | Valeur |
|----------|--------|
| ID | T310 |
| Action | RGPD-004 |
| Type | code |
| Priorite | P2 |
| Effort | S (1h) |
| Depends | T309 |
| Blocks | - |

**Instructions :**
1. Ajouter option `?format=activitypub`
2. Retourner OrderedCollection compatible

**Done Criteria :**
- [ ] Export compatible ActivityPub

---

### T311 - Documenter rotation secrets

| Attribut | Valeur |
|----------|--------|
| ID | T311 |
| Action | FIX-011 |
| Type | doc |
| Priorite | P2 |
| Effort | S (1h) |
| Depends | - |
| Blocks | T312 |
| Parallele | Oui |

**Instructions :**
1. Creer `docs/security/secret-rotation.md`
2. Documenter procedure pour chaque secret

**Done Criteria :**
- [ ] Procedure documentee

---

### T312 - Documenter politique retention

| Attribut | Valeur |
|----------|--------|
| ID | T312 |
| Action | RGPD-005 |
| Type | doc |
| Priorite | P2 |
| Effort | S (1h) |
| Depends | T311 |
| Blocks | T313 |

**Instructions :**
1. Creer `docs/legal/data-retention.md`
2. Definir durees par type de donnee

**Done Criteria :**
- [ ] Politique definie

---

### T313 - Verifier Phase 3

| Attribut | Valeur |
|----------|--------|
| ID | T313 |
| Action | - |
| Type | test |
| Priorite | P2 |
| Effort | XS (30min) |
| Depends | T303, T308, T310, T312 |
| Blocks | Phase 4 |

**Instructions :**
1. Executer `cargo test`
2. Verifier signatures fonctionnelles
3. Verifier export donnees

**Done Criteria :**
- [ ] Phase 3 complete
- [ ] Score >= 75%

---

## Phase 4 : Conformite

### T401 - Ajouter champ federation_enabled

| Attribut | Valeur |
|----------|--------|
| ID | T401 |
| Action | RGPD-007 |
| Type | code |
| Priorite | P3 |
| Effort | S (1h) |
| Depends | Phase 3 |
| Blocks | T402 |

**Instructions :**
1. Migration: ajouter `federation_enabled BOOLEAN DEFAULT true` a users
2. Mettre a jour model User

**Done Criteria :**
- [ ] Champ ajoute
- [ ] Default true

---

### T402 - API toggle federation

| Attribut | Valeur |
|----------|--------|
| ID | T402 |
| Action | RGPD-007 |
| Type | code |
| Priorite | P3 |
| Effort | S (2h) |
| Depends | T401 |
| Blocks | T403 |

**Instructions :**
1. Ajouter `PATCH /api/v1/users/me/federation`
2. Permettre toggle on/off

**Done Criteria :**
- [ ] API fonctionnelle

---

### T403 - Filtrer federation si disabled

| Attribut | Valeur |
|----------|--------|
| ID | T403 |
| Action | RGPD-007 |
| Type | code |
| Priorite | P3 |
| Effort | M (4h) |
| Depends | T402 |
| Blocks | - |

**Instructions :**
1. Dans federation jobs, verifier federation_enabled
2. Ne pas emettre activites si disabled
3. Retourner 404 sur profil Actor si disabled

**Done Criteria :**
- [ ] User peut se desinscrire de federation

---

### T404 - Creer job cleanup

| Attribut | Valeur |
|----------|--------|
| ID | T404 |
| Action | RGPD-008 |
| Type | code |
| Priorite | P3 |
| Effort | M (3h) |
| Depends | T312 |
| Blocks | T405 |

**Instructions :**
1. Creer `src/jobs/cleanup.rs`
2. Implementer purge selon politique retention
3. Scheduler job quotidien

**Done Criteria :**
- [ ] Job cree et schedule

---

### T405 - Tester purge

| Attribut | Valeur |
|----------|--------|
| ID | T405 |
| Action | RGPD-008 |
| Type | test |
| Priorite | P3 |
| Effort | S (1h) |
| Depends | T404 |
| Blocks | - |

**Instructions :**
1. Creer donnees de test expirees
2. Executer purge
3. Verifier suppression

**Done Criteria :**
- [ ] Purge fonctionne correctement

---

### T406 - Emettre Delete federe

| Attribut | Valeur |
|----------|--------|
| ID | T406 |
| Action | RGPD-009 |
| Type | code |
| Priorite | P3 |
| Effort | M (4h) |
| Depends | T403 |
| Blocks | - |

**Instructions :**
1. Dans UserService::delete, emettre activite Delete
2. Envoyer a tous followers
3. Logger emission

**Done Criteria :**
- [ ] Delete emis a suppression user
- [ ] Followers notifies

---

## Ordre d'Execution Recommande

### Semaine 1 (Phase 1 - P0)

```
Jour 1:
  T001 (5min)  ─────> T002 (30min) ─────> T003 (2min)
                          │
                          v
                      T006 (5min)

Jour 1-2:
  T003 ─────> T004 (15min) ─────> T005 (10min)
                                      │
                                      v
  T006 ─────> T007 (5min) ─────> Phase 1 Complete
```

### Semaine 2-3 (Phase 2 - P1)

```
Parallele Groupe A:          Parallele Groupe B:
  T101 ──> T102 ──> T103        T104
  T105 ──> T106 ──> T107        T109
                    │           T110 ──> T111
                    v           T112
                  T108
```

### Semaine 4-8 (Phase 3 - P2)

```
T301 ──> T302 ──> T303

Parallele:
  T304 ──> T305 ──> T306 ──> T307 ──> T308
  T309 ──> T310
  T311 ──> T312

T313 (validation finale)
```

### Semaine 9-12 (Phase 4 - P3)

```
T401 ──> T402 ──> T403
                    │
T404 ──> T405       │
                    v
                  T406
```

---

## Progression

```
Phase 1: [████████████████████]   7/7   (100%)
Phase 2: [████████████████████]  12/12  (100%)
Phase 3: [████████████████████]  13/13  (100%)
Phase 4: [████████████████████]   6/6   (100%)
─────────────────────────────────────────────
Total:   [████████████████████]  38/38  (100%)
```

---

## Prochaine Etape

```bash
# Implementer les taches
/osk-implement baseline

# Marquer une tache complete
# (mettre a jour ce fichier manuellement ou via /osk-progress)
```
