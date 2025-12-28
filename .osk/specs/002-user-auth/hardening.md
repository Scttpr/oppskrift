# Hardening Guide - User Authentication

> Generé par `/osk-harden 002` le 2025-12-26
> Principes V (Secrets), VI (Logging), VII (Patching)

## Résumé Exécutif

| Principe | État | Actions |
|----------|------|---------|
| V. Secrets | ⚠️ À améliorer | 3 corrections, 2 règles gitleaks |
| VI. Logging | ✅ Base existante | 20 événements à implémenter |
| VII. Patching | ✅ OK | Maintenir cargo-audit en CI |

---

## Principe V - Gestion des Secrets

### SEC-V-001 : Inventaire des Secrets

| ID | Secret | Type | Criticité | Rotation | Stockage |
|----|--------|------|-----------|----------|----------|
| SEC-001 | `JWT_SECRET` | HMAC-SHA256 | **CRITIQUE** | 90j | Env var |
| SEC-002 | `TOTP_ENCRYPTION_KEY` | AES-256-GCM | **CRITIQUE** | Annuel | Env var |
| SEC-003 | `DATABASE_URL` | Connection string | HIGH | 30j | Env var |
| SEC-004 | `SMTP_PASSWORD` | Service credential | HIGH | 90j | Env var |
| SEC-005 | `S3_SECRET_ACCESS_KEY` | AWS credential | HIGH | 90j | Env var |
| SEC-006 | Session tokens | Bearer | MEDIUM | 7j auto | DB (hashed) |
| SEC-007 | Password reset tokens | One-time | MEDIUM | 1h auto | DB (hashed) |
| SEC-008 | Email confirm tokens | One-time | MEDIUM | 24h auto | DB (hashed) |
| SEC-009 | Recovery codes | Backup auth | HIGH | Single-use | DB (bcrypt) |

### SEC-V-002 : Validation des Secrets (Config Module)

**Fichier** : `src/lib/config.rs`

```rust
//! Validated configuration module
//! All secrets MUST be validated at startup - no fallbacks in production

use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnv(String),
    #[error("Invalid secret length for {0}: expected at least {1} bytes")]
    InvalidSecretLength(String, usize),
    #[error("Production environment requires all secrets to be set")]
    ProductionRequiresSecrets,
}

#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub totp_encryption_key: [u8; 32],
    pub session_expiry_days: u32,
    pub lockout_duration_minutes: u32,
    pub rate_limit_login_per_ip: u32,
    pub rate_limit_login_per_account: u32,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let is_production = env::var("RUST_ENV")
            .map(|v| v == "production")
            .unwrap_or(false);

        // JWT_SECRET - REQUIRED, no fallback
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| ConfigError::MissingEnv("JWT_SECRET".into()))?;

        if jwt_secret.len() < 32 {
            return Err(ConfigError::InvalidSecretLength("JWT_SECRET".into(), 32));
        }

        // TOTP_ENCRYPTION_KEY - REQUIRED for 2FA
        let totp_key_hex = env::var("TOTP_ENCRYPTION_KEY")
            .map_err(|_| ConfigError::MissingEnv("TOTP_ENCRYPTION_KEY".into()))?;

        let totp_key_bytes = hex::decode(&totp_key_hex)
            .map_err(|_| ConfigError::InvalidSecretLength("TOTP_ENCRYPTION_KEY".into(), 32))?;

        if totp_key_bytes.len() != 32 {
            return Err(ConfigError::InvalidSecretLength("TOTP_ENCRYPTION_KEY".into(), 32));
        }

        let mut totp_encryption_key = [0u8; 32];
        totp_encryption_key.copy_from_slice(&totp_key_bytes);

        // Optional with safe defaults
        let session_expiry_days = env::var("SESSION_EXPIRY_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7);

        let lockout_duration_minutes = env::var("LOCKOUT_DURATION_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);

        let rate_limit_login_per_ip = env::var("RATE_LIMIT_LOGIN_PER_IP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let rate_limit_login_per_account = env::var("RATE_LIMIT_LOGIN_PER_ACCOUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        // Production validation
        if is_production {
            // Ensure no development patterns in secrets
            if jwt_secret.contains("dev") || jwt_secret.contains("test") {
                return Err(ConfigError::ProductionRequiresSecrets);
            }
        }

        Ok(Self {
            jwt_secret,
            totp_encryption_key,
            session_expiry_days,
            lockout_duration_minutes,
            rate_limit_login_per_ip,
            rate_limit_login_per_account,
        })
    }
}

#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

impl SmtpConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: env::var("SMTP_HOST")
                .map_err(|_| ConfigError::MissingEnv("SMTP_HOST".into()))?,
            port: env::var("SMTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(587),
            username: env::var("SMTP_USER")
                .map_err(|_| ConfigError::MissingEnv("SMTP_USER".into()))?,
            password: env::var("SMTP_PASSWORD")
                .map_err(|_| ConfigError::MissingEnv("SMTP_PASSWORD".into()))?,
            from: env::var("EMAIL_FROM")
                .unwrap_or_else(|_| "noreply@oppskrift.local".into()),
        })
    }
}
```

### SEC-V-003 : Génération de Secrets

**Script** : `scripts/generate-secrets.sh`

```bash
#!/bin/bash
# Generate cryptographically secure secrets for Oppskrift

set -euo pipefail

echo "# Oppskrift Auth Secrets - Generated $(date -I)"
echo "# DO NOT COMMIT THIS FILE"
echo ""

# JWT_SECRET - 256-bit (32 bytes) minimum
echo "JWT_SECRET=$(openssl rand -base64 48 | tr -d '\n')"

# TOTP_ENCRYPTION_KEY - exactly 256-bit (32 bytes) hex-encoded
echo "TOTP_ENCRYPTION_KEY=$(openssl rand -hex 32)"

# Session token example (for reference, generated per-session)
echo ""
echo "# Example session token format (generated per login):"
echo "# SESSION_TOKEN=$(openssl rand -hex 32)"
```

### SEC-V-004 : Règles Gitleaks Additionnelles

**Ajouter à** : `.gitleaks.toml`

```toml
[[rules]]
id = "totp-encryption-key"
description = "TOTP Encryption Key"
regex = '''TOTP_ENCRYPTION_KEY\s*=\s*["']?[a-fA-F0-9]{64}["']?'''
keywords = ["TOTP_ENCRYPTION_KEY", "totp_encryption"]

[[rules]]
id = "smtp-password"
description = "SMTP Password"
regex = '''SMTP_PASSWORD\s*=\s*["']?[^"'\s]+["']?'''
keywords = ["SMTP_PASSWORD", "smtp_pass"]

[[rules]]
id = "argon2-hash"
description = "Argon2 Password Hash (should never be in code)"
regex = '''\$argon2(id|i|d)\$v=\d+\$m=\d+,t=\d+,p=\d+\$[A-Za-z0-9+/]+\$[A-Za-z0-9+/]+'''
keywords = ["argon2"]
```

### SEC-V-005 : Politique de Rotation

| Type de Secret | Fréquence | Procédure |
|----------------|-----------|-----------|
| JWT_SECRET | 90 jours | Rotation sans downtime (double-key) |
| TOTP_ENCRYPTION_KEY | Annuel | Re-chiffrement des secrets TOTP |
| DATABASE_URL | 30 jours | Rotation credentials PostgreSQL |
| SMTP_PASSWORD | 90 jours | Rotation via provider |
| Session tokens | 7 jours | Auto-expiration |
| Reset tokens | 1 heure | Auto-expiration |

**JWT Rotation Strategy** :

```rust
// Support dual JWT secrets during rotation period
pub struct JwtConfig {
    pub current_secret: String,
    pub previous_secret: Option<String>, // Valid for 24h after rotation
}

impl JwtConfig {
    pub fn validate_token(&self, token: &str) -> Result<Claims, Error> {
        // Try current secret first
        if let Ok(claims) = validate_with_secret(token, &self.current_secret) {
            return Ok(claims);
        }

        // Fallback to previous secret during rotation window
        if let Some(ref prev) = self.previous_secret {
            if let Ok(claims) = validate_with_secret(token, prev) {
                // Log that old secret was used (rotation monitoring)
                tracing::info!("JWT validated with previous secret - rotation in progress");
                return Ok(claims);
            }
        }

        Err(Error::InvalidToken)
    }
}
```

### SEC-V-006 : Checklist Secrets

- [ ] `JWT_SECRET` validé (32+ caractères, pas de pattern dev/test)
- [ ] `TOTP_ENCRYPTION_KEY` généré (64 hex = 32 bytes)
- [ ] `SMTP_PASSWORD` configuré (pas de fallback)
- [ ] Gitleaks rules ajoutées (TOTP, SMTP, Argon2)
- [ ] Script génération secrets créé
- [ ] Rotation policy documentée
- [ ] `.env.example` mis à jour (sans vraies valeurs)

---

## Principe VI - Audit Logging

### SEC-VI-001 : Événements Authentification

**Extension du module** : `src/lib/audit.rs`

```rust
//! Auth-specific audit events
//! All security-relevant authentication actions must be logged

use super::AuditEvent;
use std::net::IpAddr;
use uuid::Uuid;

/// Authentication audit events
pub mod auth {
    use super::*;

    // ─────────────────────────────────────────────────────────────────
    // Registration Events
    // ─────────────────────────────────────────────────────────────────

    pub fn register_success(user_id: Uuid, email: &str, ip: IpAddr) {
        AuditEvent::new("auth.register.success")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .with_metadata("email_domain", email.split('@').last().unwrap_or("unknown"))
            .log();
    }

    pub fn register_failure(email: &str, reason: &str, ip: IpAddr) {
        AuditEvent::new("auth.register.failure")
            .warn()
            .with_ip(&ip.to_string())
            .with_metadata("email_domain", email.split('@').last().unwrap_or("unknown"))
            .with_metadata("reason", reason)
            .log();
    }

    // ─────────────────────────────────────────────────────────────────
    // Login Events
    // ─────────────────────────────────────────────────────────────────

    pub fn login_success(user_id: Uuid, ip: IpAddr, user_agent: &str) {
        AuditEvent::new("auth.login.success")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .with_metadata("user_agent", user_agent)
            .log();
    }

    pub fn login_failure(email: &str, reason: &str, ip: IpAddr) {
        AuditEvent::new("auth.login.failure")
            .warn()
            .with_ip(&ip.to_string())
            .with_metadata("email_domain", email.split('@').last().unwrap_or("unknown"))
            .with_metadata("reason", reason)
            .log();
    }

    pub fn login_locked(user_id: Uuid, ip: IpAddr, locked_until: &str) {
        AuditEvent::new("auth.login.locked")
            .warn()
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .with_metadata("locked_until", locked_until)
            .log();
    }

    pub fn logout(user_id: Uuid, session_id: Uuid) {
        AuditEvent::new("auth.logout")
            .with_user(user_id)
            .with_target("session", session_id)
            .log();
    }

    // ─────────────────────────────────────────────────────────────────
    // Password Events
    // ─────────────────────────────────────────────────────────────────

    pub fn password_reset_request(email: &str, ip: IpAddr) {
        AuditEvent::new("auth.password.reset.request")
            .with_ip(&ip.to_string())
            .with_metadata("email_domain", email.split('@').last().unwrap_or("unknown"))
            .log();
    }

    pub fn password_reset_complete(user_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.password.reset.complete")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .log();
    }

    pub fn password_change(user_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.password.change")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .log();
    }

    // ─────────────────────────────────────────────────────────────────
    // Email Events
    // ─────────────────────────────────────────────────────────────────

    pub fn email_change(user_id: Uuid, old_domain: &str, new_domain: &str, ip: IpAddr) {
        AuditEvent::new("auth.email.change")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .with_metadata("old_domain", old_domain)
            .with_metadata("new_domain", new_domain)
            .log();
    }

    pub fn email_confirmed(user_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.email.confirmed")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .log();
    }

    // ─────────────────────────────────────────────────────────────────
    // 2FA Events
    // ─────────────────────────────────────────────────────────────────

    pub fn totp_enable(user_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.2fa.enable")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .log();
    }

    pub fn totp_disable(user_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.2fa.disable")
            .warn()
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .log();
    }

    pub fn recovery_code_used(user_id: Uuid, codes_remaining: u8, ip: IpAddr) {
        AuditEvent::new("auth.2fa.recovery.used")
            .warn()
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .with_metadata("codes_remaining", &codes_remaining.to_string())
            .log();
    }

    // ─────────────────────────────────────────────────────────────────
    // Session Events
    // ─────────────────────────────────────────────────────────────────

    pub fn session_revoke(user_id: Uuid, session_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.session.revoke")
            .with_user(user_id)
            .with_target("session", session_id)
            .with_ip(&ip.to_string())
            .log();
    }

    pub fn session_revoke_all(user_id: Uuid, count: u32, ip: IpAddr) {
        AuditEvent::new("auth.session.revoke.all")
            .warn()
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .with_metadata("sessions_revoked", &count.to_string())
            .log();
    }

    // ─────────────────────────────────────────────────────────────────
    // Account Deletion Events (RGPD Art. 17)
    // ─────────────────────────────────────────────────────────────────

    pub fn delete_request(user_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.account.delete.request")
            .warn()
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .log();
    }

    pub fn delete_cancel(user_id: Uuid, ip: IpAddr) {
        AuditEvent::new("auth.account.delete.cancel")
            .with_user(user_id)
            .with_ip(&ip.to_string())
            .log();
    }

    pub fn delete_execute(user_id: Uuid, recipes_orphaned: u32) {
        AuditEvent::new("auth.account.delete.execute")
            .warn()
            .with_user(user_id)
            .with_metadata("recipes_orphaned", &recipes_orphaned.to_string())
            .log();
    }

    // ─────────────────────────────────────────────────────────────────
    // Security Events
    // ─────────────────────────────────────────────────────────────────

    pub fn rate_limit_exceeded(ip: IpAddr, endpoint: &str) {
        AuditEvent::new("auth.rate.limit.exceeded")
            .warn()
            .with_ip(&ip.to_string())
            .with_metadata("endpoint", endpoint)
            .log();
    }

    pub fn suspicious_activity(user_id: Option<Uuid>, ip: IpAddr, reason: &str) {
        let mut event = AuditEvent::new("auth.suspicious.activity")
            .error()
            .with_ip(&ip.to_string())
            .with_metadata("reason", reason);

        if let Some(uid) = user_id {
            event = event.with_user(uid);
        }

        event.log();
    }
}
```

### SEC-VI-002 : Format JSON Structuré

**Exemple de sortie** :

```json
{
  "timestamp": "2025-12-26T14:30:00.123Z",
  "event": "auth.login.failure",
  "level": "warn",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "service": "oppskrift",
  "user_id": null,
  "ip": "192.168.1.100",
  "target_type": null,
  "target_id": null,
  "metadata": {
    "email_domain": "example.com",
    "reason": "invalid_password"
  }
}
```

### SEC-VI-003 : Tracing Configuration

**Fichier** : `src/main.rs`

```rust
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,oppskrift=debug"));

    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    if is_production {
        // Production: JSON format for log aggregation
        let json_layer = fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_file(false)
            .with_line_number(false);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(json_layer)
            .init();
    } else {
        // Development: Human-readable format
        let fmt_layer = fmt::layer()
            .pretty()
            .with_target(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }
}
```

### SEC-VI-004 : Rétention des Logs

**Configuration RGPD** :

| Type de Log | Rétention | Justification |
|-------------|-----------|---------------|
| `auth.login.*` | 1 an | Art. 32 - Détection intrusion |
| `auth.register.*` | 3 ans | Base légale (contrat) |
| `auth.account.delete.*` | 5 ans | Preuve RGPD Art. 17 |
| `auth.suspicious.*` | 2 ans | Forensics |
| `auth.2fa.*` | 1 an | Audit sécurité |
| `auth.session.*` | 90 jours | Debugging |

### SEC-VI-005 : Alertes Recommandées

**Prometheus/Grafana Rules** :

```yaml
# prometheus-rules.yml
groups:
  - name: oppskrift-auth-alerts
    rules:
      # Brute force detection
      - alert: BruteForceAttempt
        expr: |
          sum(rate(oppskrift_auth_login_failure_total[5m])) by (ip) > 10
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Possible brute force from {{ $labels.ip }}"

      # Account lockout spike
      - alert: AccountLockoutSpike
        expr: |
          sum(increase(oppskrift_auth_login_locked_total[15m])) > 50
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Unusual number of account lockouts"

      # 2FA disable spike (potential account compromise)
      - alert: TwoFactorDisableSpike
        expr: |
          sum(increase(oppskrift_auth_2fa_disable_total[1h])) > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Unusual 2FA disables - possible account compromise"

      # Rate limit exceeded
      - alert: RateLimitExceeded
        expr: |
          sum(rate(oppskrift_auth_rate_limit_exceeded_total[5m])) by (endpoint) > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Rate limit exceeded on {{ $labels.endpoint }}"
```

### SEC-VI-006 : Checklist Logging

- [ ] Module `audit::auth` implémenté
- [ ] Tracing JSON en production
- [ ] Tous les 20 événements intégrés aux handlers
- [ ] Rétention configurée selon RGPD
- [ ] Alertes Prometheus définies
- [ ] Pas de données personnelles directes dans les logs (email domain only)

---

## Principe VII - Patch Management

### SEC-VII-001 : État Actuel des Dépendances

```
cargo-audit v0.22.0
Scanning Cargo.lock: 520 crates
Vulnerabilities: 0 (as of 2025-12-26)
```

### SEC-VII-002 : SLA de Patching

| Sévérité CVSS | Délai Max | Action |
|---------------|-----------|--------|
| Critical (9.0-10.0) | 48h | Hotfix immédiat |
| High (7.0-8.9) | 7 jours | Sprint courant |
| Medium (4.0-6.9) | 30 jours | Prochain sprint |
| Low (0.1-3.9) | 90 jours | Maintenance |

### SEC-VII-003 : Configuration Dependabot (Existante)

**Fichier** : `.github/dependabot.yml` - ✅ Déjà configuré

Améliorations recommandées :

```yaml
# Ajouter après la section cargo existante
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "daily"  # Changed from weekly for security
      time: "06:00"
      timezone: "Europe/Paris"
    open-pull-requests-limit: 10
    commit-message:
      prefix: "chore(deps)"
    labels:
      - "dependencies"
      - "security"
    ignore:
      # Ignore patch updates for low-risk deps
      - dependency-name: "serde"
        update-types: ["version-update:semver-patch"]
    # Security updates always get priority
    allow:
      - dependency-type: "all"
```

### SEC-VII-004 : CI Security Pipeline

**Fichier** : `.github/workflows/security.yml`

```yaml
name: Security Checks

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]
  schedule:
    # Daily security scan at 6 AM UTC
    - cron: '0 6 * * *'

jobs:
  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            ~/.cargo/advisory-db
          key: ${{ runner.os }}-cargo-audit-${{ hashFiles('**/Cargo.lock') }}

      - name: Install cargo-audit
        run: cargo install cargo-audit --locked

      - name: Run cargo audit
        run: |
          cargo audit --deny warnings --deny unmaintained --deny yanked

      - name: Check for RUSTSEC advisories
        run: |
          # Fail on CVSS >= 7.0 (High/Critical)
          cargo audit --json | jq -e '.vulnerabilities.list | length == 0' || {
            echo "::error::Security vulnerabilities found!"
            exit 1
          }

  gitleaks:
    name: Secret Scan
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Run Gitleaks
        uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  clippy-security:
    name: Static Analysis
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Run Clippy with security lints
        run: |
          cargo clippy --all-features -- \
            -D clippy::unwrap_used \
            -D clippy::expect_used \
            -D unsafe_code \
            -W clippy::todo \
            -W clippy::unimplemented
```

### SEC-VII-005 : Procédure CVE Critique

```
┌─────────────────────────────────────────────────────────────────┐
│                    PROCÉDURE CVE CRITIQUE                       │
│                        (CVSS ≥ 9.0)                             │
├─────────────────────────────────────────────────────────────────┤
│ T+0h    │ Alerte Dependabot/cargo-audit                        │
│ T+1h    │ Évaluation impact sur Oppskrift                      │
│ T+4h    │ PR avec upgrade si patch disponible                  │
│ T+8h    │ Tests sur staging                                    │
│ T+24h   │ Déploiement production                               │
│ T+48h   │ Deadline absolue                                     │
├─────────────────────────────────────────────────────────────────┤
│ Si pas de patch disponible :                                    │
│ - Évaluer workaround (disable feature, firewall rule)          │
│ - Documenter dans SECURITY.md                                   │
│ - Monitorer releases upstream                                   │
└─────────────────────────────────────────────────────────────────┘
```

### SEC-VII-006 : Checklist Patching

- [x] cargo-audit installé (v0.22.0)
- [x] Dependabot configuré
- [x] Gitleaks en pre-commit
- [ ] CI security pipeline créé
- [ ] SLA patching documenté
- [ ] Procédure CVE critique documentée

---

## Actions Immédiates

### Priorité 1 (Avant implémentation)

1. **Mettre à jour `.gitleaks.toml`** avec les 3 nouvelles règles
2. **Créer `scripts/generate-secrets.sh`**
3. **Créer `.github/workflows/security.yml`**

### Priorité 2 (Pendant implémentation)

4. **Implémenter `AuthConfig` et `SmtpConfig`** dans `src/lib/config.rs`
5. **Étendre `src/lib/audit.rs`** avec le module `auth`
6. **Intégrer les appels audit** dans chaque handler auth

### Priorité 3 (Post-implémentation)

7. **Configurer alertes** Prometheus/Grafana
8. **Mettre à jour Dependabot** pour scans quotidiens
9. **Documenter procédure CVE** dans SECURITY.md

---

## Références

- [OWASP ASVS v4.0 - V2 Authentication](https://owasp.org/www-project-application-security-verification-standard/)
- [RGPD Article 17 - Droit à l'effacement](https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre3#Article17)
- [RustSec Advisory Database](https://rustsec.org/)
- [Gitleaks Rules](https://github.com/gitleaks/gitleaks#rules)
