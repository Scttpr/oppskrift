# Mesures de Durcissement - Baseline

> Généré par `/osk-harden baseline` le 2025-12-26
> Principes V, VI, VII

## Résumé Exécutif

| Principe | Statut | Couverture | Actions |
|----------|--------|------------|---------|
| V. Secrets | CRITIQUE | 20% | 5 secrets, 3 vulnérabilités |
| VI. Logging | À RISQUE | 25% | 14 événements à implémenter |
| VII. Patching | OK | 60% | 0 CVE, config à compléter |

---

## V. Gestion des Secrets

### Inventaire

| ID | Secret | Type | Sensibilité | Rotation |
|----|--------|------|-------------|----------|
| SECRET-SYS-001 | `DATABASE_URL` | db_credential | Critical | 90j |
| SECRET-SYS-002 | `JWT_SECRET` | encryption_key | Critical | 90j |
| SECRET-SYS-003 | `S3_ACCESS_KEY_ID` | api_key | High | 90j |
| SECRET-SYS-004 | `S3_SECRET_ACCESS_KEY` | api_key | Critical | 90j |
| SECRET-SYS-005 | ActivityPub Private Key | certificate | Critical | 1an |

### Vulnérabilités Détectées

#### VULN-BASELINE-001 - JWT Secret Fallback

**Fichier:** `src/api/middleware/auth.rs:87`

**Code actuel:**
```rust
let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret".to_string());
```

**Code corrigé:**
```rust
let secret = std::env::var("JWT_SECRET")
    .expect("JWT_SECRET environment variable must be set");
```

#### VULN-BASELINE-002 - Clé Privée Placeholder

**Fichier:** `src/jobs/federation.rs:127`

**Code actuel:**
```rust
"placeholder_private_key".to_string(), // TODO: Get from DB
```

**Solution:** Créer une table `user_keys` et récupérer la clé privée de l'utilisateur.

### Configuration Requise

#### Module de Validation des Secrets

Créer `src/lib/config.rs`:

```rust
//! Configuration validation module
//! Ensures all required secrets are present at startup

use std::env;

/// Configuration loaded from environment
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
    /// Load and validate configuration from environment
    /// Panics if required variables are missing or invalid
    pub fn from_env() -> Self {
        // Required secrets - panic if missing
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let jwt_secret = env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set");

        // Validate JWT_SECRET length
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters");
        }

        let s3_bucket = env::var("S3_BUCKET")
            .expect("S3_BUCKET must be set");

        // Optional with defaults
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
            database_url,
            jwt_secret,
            base_url,
            s3_bucket,
            s3_region,
            s3_endpoint,
            host,
            port,
        }
    }
}
```

### Pre-commit Hook (Gitleaks)

Créer `.pre-commit-config.yaml`:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/gitleaks/gitleaks
    rev: v8.18.0
    hooks:
      - id: gitleaks
        name: Detect hardcoded secrets
        description: Detect hardcoded secrets using Gitleaks
        entry: gitleaks protect --staged --verbose
```

Créer `.gitleaks.toml`:

```toml
# .gitleaks.toml
title = "Oppskrift Gitleaks Configuration"

[allowlist]
description = "Allowed patterns"
paths = [
    ".env.example",
    "docker-compose.yml",  # Contains example values only
]

[[rules]]
id = "jwt-secret"
description = "JWT Secret"
regex = '''JWT_SECRET\s*=\s*["\']?[^"\'\s]+["\']?'''
keywords = ["jwt_secret", "JWT_SECRET"]

[[rules]]
id = "database-url"
description = "Database URL with credentials"
regex = '''postgres://[^:]+:[^@]+@'''
keywords = ["DATABASE_URL", "postgres://"]

[[rules]]
id = "s3-secret"
description = "S3 Secret Access Key"
regex = '''S3_SECRET_ACCESS_KEY\s*=\s*["\']?[^"\'\s]+["\']?'''
keywords = ["S3_SECRET", "secret_access_key"]
```

### Checklist Principe V

- [ ] SEC-V-001 : Tous les secrets requis sont validés au démarrage
- [ ] SEC-V-002 : Aucun fallback dangereux dans le code
- [ ] SEC-V-003 : Gitleaks configuré en pre-commit
- [ ] SEC-V-004 : Secrets documentés dans .env.example
- [ ] SEC-V-005 : Politique de rotation documentée

---

## VI. Traçabilité et Audit

### Événements à Logger

| Événement | Niveau | Rétention | Alerte |
|-----------|--------|-----------|--------|
| `auth.login.success` | info | 1an | Non |
| `auth.login.failure` | warn | 1an | Oui |
| `auth.token.invalid` | warn | 1an | Oui |
| `auth.token.expired` | info | 90j | Non |
| `user.create` | info | 3ans | Non |
| `user.update` | info | 3ans | Non |
| `user.delete` | info | 3ans | Non |
| `user.data.export` | info | 3ans | Non |
| `recipe.create` | info | 1an | Non |
| `recipe.update` | info | 1an | Non |
| `recipe.delete` | info | 1an | Non |
| `federation.inbox.received` | info | 90j | Non |
| `federation.inbox.rejected` | warn | 1an | Oui |
| `federation.signature.invalid` | error | 1an | Oui |
| `security.rate_limit.exceeded` | warn | 90j | Oui |
| `security.access_denied` | warn | 1an | Oui |

### Format de Log Structuré

```rust
//! Audit logging module
//! src/lib/audit.rs

use serde::Serialize;
use tracing::info;
use uuid::Uuid;

/// Structured audit event
#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event: String,
    pub level: String,
    pub trace_id: Uuid,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AuditEvent {
    pub fn new(event: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            event: event.to_string(),
            level: "info".to_string(),
            trace_id: Uuid::new_v4(),
            service: "oppskrift".to_string(),
            user_id: None,
            ip: None,
            target_type: None,
            target_id: None,
            metadata: None,
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_ip(mut self, ip: &str) -> Self {
        self.ip = Some(ip.to_string());
        self
    }

    pub fn with_target(mut self, target_type: &str, target_id: Uuid) -> Self {
        self.target_type = Some(target_type.to_string());
        self.target_id = Some(target_id);
        self
    }

    pub fn warn(mut self) -> Self {
        self.level = "warn".to_string();
        self
    }

    pub fn error(mut self) -> Self {
        self.level = "error".to_string();
        self
    }

    /// Log the event
    pub fn log(self) {
        let json = serde_json::to_string(&self).unwrap_or_default();
        match self.level.as_str() {
            "warn" => tracing::warn!(audit = %json, "audit event"),
            "error" => tracing::error!(audit = %json, "audit event"),
            _ => tracing::info!(audit = %json, "audit event"),
        }
    }
}

// Usage examples:
// AuditEvent::new("auth.login.success").with_user(user_id).with_ip(&ip).log();
// AuditEvent::new("auth.login.failure").with_ip(&ip).warn().log();
// AuditEvent::new("user.delete").with_user(actor_id).with_target("user", target_id).log();
```

### Alertes Recommandées

| Règle | Condition | Sévérité |
|-------|-----------|----------|
| Brute Force Login | 5+ `auth.login.failure` même IP en 5min | Critical |
| Token Flood | 10+ `auth.token.invalid` en 1min | High |
| Federation Attack | 20+ `federation.inbox.rejected` même source en 10min | High |
| Signature Forgery | Tout `federation.signature.invalid` | Critical |
| Rate Limit Abuse | 50+ `security.rate_limit.exceeded` même IP en 1h | Medium |
| Privilege Escalation | `security.access_denied` sur ressources admin | Critical |

### Configuration Tracing JSON

Modifier `src/main.rs`:

```rust
// Replace existing tracing setup with JSON format for production
use tracing_subscriber::fmt::format::FmtSpan;

fn setup_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Use JSON format in production
    if std::env::var("RUST_ENV").unwrap_or_default() == "production" {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_span_events(FmtSpan::CLOSE)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .init();
    }
}
```

### Checklist Principe VI

- [ ] SEC-VI-001 : Format JSON structuré en production
- [ ] SEC-VI-002 : Tous les événements d'authentification loggés
- [ ] SEC-VI-003 : Toutes les actions CRUD loggées
- [ ] SEC-VI-004 : Événements fédération loggés
- [ ] SEC-VI-005 : Alertes configurées sur événements critiques
- [ ] SEC-VI-006 : Rétention conforme RGPD (3 ans données personnelles)

---

## VII. Patch Management

### État des Dépendances

```
cargo audit: 0 vulnérabilités connues
```

### SLA de Patching

| Sévérité | Délai | Action |
|----------|-------|--------|
| Critical (CVSS 9.0+) | 48h | Patch immédiat, déploiement d'urgence |
| High (CVSS 7.0-8.9) | 7j | Patch planifié, sprint courant |
| Medium (CVSS 4.0-6.9) | 30j | Prochain cycle de maintenance |
| Low (CVSS < 4.0) | 90j | Backlog de maintenance |

### Configuration Dependabot

Créer `.github/dependabot.yml`:

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    open-pull-requests-limit: 5
    groups:
      rust-minor:
        patterns:
          - "*"
        update-types:
          - "minor"
          - "patch"
    labels:
      - "dependencies"
      - "rust"
    commit-message:
      prefix: "chore(deps)"

  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    labels:
      - "dependencies"
      - "ci"
```

### Procédure CVE Critique

```markdown
## Procédure d'Urgence CVE Critique

1. **Détection** (T+0)
   - Alerte cargo-audit CI ou advisories@rust-lang.org
   - Évaluer impact sur Oppskrift

2. **Évaluation** (T+2h)
   - Vérifier si la vulnérabilité est exploitable dans notre contexte
   - Si exploitable: passer en mode incident
   - Si non exploitable: documenter et planifier patch standard

3. **Correction** (T+24h max)
   - Créer branche `hotfix/cve-XXXX-XXXX`
   - Mettre à jour dépendance
   - Exécuter tests complets
   - Review de sécurité

4. **Déploiement** (T+48h max)
   - Merge et tag release
   - Déploiement production
   - Vérification post-déploiement

5. **Post-mortem** (T+7j)
   - Documenter l'incident
   - Améliorer détection si nécessaire
```

### Checklist Principe VII

- [ ] SEC-VII-001 : cargo-audit dans CI (déjà fait)
- [ ] SEC-VII-002 : Dependabot configuré
- [ ] SEC-VII-003 : SLA de patching documenté
- [ ] SEC-VII-004 : Procédure CVE critique documentée
- [ ] SEC-VII-005 : Notifications sur nouvelles CVE activées

---

## Actions Requises

### Priorité P0 - Critique (48h)

| ID | Action | Fichier | Effort |
|----|--------|---------|--------|
| FIX-001 | Remplacer fallback JWT_SECRET par `expect()` | `src/api/middleware/auth.rs:87` | 5min |
| FIX-002 | Créer module Config avec validation | `src/lib/config.rs` | 1h |
| FIX-003 | Intégrer Config dans main.rs | `src/main.rs` | 30min |

### Priorité P1 - Important (7j)

| ID | Action | Fichier | Effort |
|----|--------|---------|--------|
| FIX-004 | Créer module audit logging | `src/lib/audit.rs` | 2h |
| FIX-005 | Configurer gitleaks pre-commit | `.pre-commit-config.yaml` | 30min |
| FIX-006 | Créer migration `user_keys` | `migrations/` | 1h |
| FIX-007 | Générer clés RSA à création user | `src/services/user_service.rs` | 2h |
| FIX-008 | Configurer Dependabot | `.github/dependabot.yml` | 15min |

### Priorité P2 - Standard (30j)

| ID | Action | Fichier | Effort |
|----|--------|---------|--------|
| FIX-009 | Implémenter logging JSON production | `src/main.rs` | 1h |
| FIX-010 | Ajouter audit events aux services | `src/services/*.rs` | 4h |
| FIX-011 | Documenter procédure rotation secrets | `docs/security/` | 2h |
| FIX-012 | Configurer alertes (Grafana/Loki) | Infrastructure | 4h |

---

## Prochaine Étape

→ Exécuter les corrections P0 immédiatement
→ `/osk-implement baseline` pour générer les fichiers de correction
