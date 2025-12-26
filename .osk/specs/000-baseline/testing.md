# Strategie de Tests de Securite - Baseline

> Genere par `/osk-specify baseline` le 2025-12-26
> Principe IV - Security Testing

## Resume

| Type | Outil | Integration | Bloquant |
|------|-------|-------------|----------|
| SAST | cargo clippy + Semgrep | CI/PR | Oui |
| DAST | OWASP ZAP | Staging | Non (alertes) |
| SCA | cargo audit + Dependabot | PR + Daily | Oui (CVSS >= 7.0) |
| Unit | cargo test | CI/PR | Oui |

---

## SAST - Analyse Statique

### Outils

| Outil | Role | Regles |
|-------|------|--------|
| `cargo clippy` | Linting Rust | Defaut + deny(warnings) |
| `cargo fmt --check` | Formatage | Rustfmt defaut |
| Semgrep | Patterns securite | rust.lang.security.* |

### Configuration Semgrep

```yaml
# .semgrep.yml
rules:
  - id: rust-hardcoded-secret
    patterns:
      - pattern: |
          $SECRET = "..."
      - metavariable-regex:
          metavariable: $SECRET
          regex: (secret|password|key|token)
    message: "Hardcoded secret detected"
    severity: ERROR

  - id: rust-sql-injection
    pattern: |
      sqlx::query($SQL)
    message: "Use query! macro for compile-time verification"
    severity: WARNING
```

### Tests SAST Specifiques

| ID | Type | Description | Regle |
|----|------|-------------|-------|
| SAST-001 | Secrets | Detection secrets hardcodes | rust-hardcoded-secret |
| SAST-002 | Injection | Detection SQL dynamique | rust-sql-injection |
| SAST-003 | Crypto | Verification RSA 2048+ | rust-weak-crypto |
| SAST-004 | Auth | Verification JWT validation | rust-jwt-validation |

### Integration CI

```yaml
# .github/workflows/security.yml
name: Security Checks

on: [push, pull_request]

jobs:
  sast:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Semgrep
        uses: returntocorp/semgrep-action@v1
        with:
          config: .semgrep.yml
```

---

## DAST - Analyse Dynamique

### Outil

**OWASP ZAP** en mode API scan

### Endpoints a Tester

| Endpoint | Methodes | Tests |
|----------|----------|-------|
| `/api/v1/auth/login` | POST | Brute force, Injection |
| `/api/v1/users/me` | GET, PATCH | Auth bypass, IDOR |
| `/api/v1/recipes` | GET, POST | Injection, Auth |
| `/api/v1/recipes/:id` | GET, PATCH, DELETE | IDOR, Auth bypass |
| `/ap/users/:id/inbox` | POST | Signature bypass, Injection |
| `/ap/inbox` | POST | Signature bypass |

### Scenarios d'Attaque

| ID | Type | Payload | Reponse Attendue |
|----|------|---------|------------------|
| DAST-001 | JWT Forgery | Token signe "dev-secret" | 401 Unauthorized |
| DAST-002 | IDOR Recipe | GET /recipes/:other_private | 404 Not Found |
| DAST-003 | SQL Injection | `' OR '1'='1` dans search | 400 Bad Request |
| DAST-004 | XSS Stored | `<script>alert(1)</script>` | Contenu sanitize |
| DAST-005 | Inbox Forgery | Activity sans signature | 401 Unauthorized |
| DAST-006 | Path Traversal | `../../../etc/passwd` | 400 Bad Request |

### Configuration ZAP

```yaml
# zap-api-scan.yaml
env:
  contexts:
    - name: "Oppskrift API"
      urls:
        - "http://localhost:3000"
      includePaths:
        - "http://localhost:3000/api/.*"
        - "http://localhost:3000/ap/.*"
      authentication:
        method: "script"
        parameters:
          script: "jwt-auth.js"

jobs:
  - type: "openapi"
    parameters:
      apiFile: "docs/openapi.json"
      targetUrl: "http://localhost:3000"

  - type: "activeScan"
    parameters:
      maxRuleDurationInMins: 10
      maxScanDurationInMins: 60
```

---

## SCA - Analyse des Dependances

### Outils

| Outil | Fonction | Frequence |
|-------|----------|-----------|
| `cargo audit` | CVE Rust | Chaque PR |
| Dependabot | Updates auto | Quotidien |
| `cargo deny` | Licences + CVE | CI |

### Politique

- **CVSS max autorise** : 6.9 (bloque >= 7.0)
- **Licences interdites** : GPL-3.0, AGPL-3.0
- **Frequence scan** : Quotidien + chaque PR

### Configuration Dependabot

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "daily"
    open-pull-requests-limit: 5
    commit-message:
      prefix: "chore(deps)"
    labels:
      - "dependencies"
      - "security"
```

### Configuration cargo-deny

```toml
# deny.toml
[advisories]
db-path = "~/.cargo/advisory-db"
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
]
deny = [
    "GPL-3.0",
    "AGPL-3.0",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"
```

### Vulnerabilites Connues

| Dependance | CVE | CVSS | Action |
|------------|-----|------|--------|
| *Scan requis* | - | - | `cargo audit` |

---

## Tests Unitaires de Securite

### Framework

- **Outil** : `cargo test`
- **Couverture cible** : 80% code securite
- **Types** : Positif, Negatif, Boundary

### Exemples de Tests

```rust
// src/lib/activitypub/signature.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_signature() {
        let header = r#"keyId="https://example.com/users/1#main-key",algorithm="rsa-sha256",headers="(request-target) host date",signature="base64...""#;
        let sig = HttpSignature::parse(header);
        assert!(sig.is_some());
    }

    #[test]
    fn test_reject_invalid_signature() {
        let header = "invalid";
        let sig = HttpSignature::parse(header);
        assert!(sig.is_none());
    }

    #[test]
    fn test_reject_expired_date() {
        // Date trop ancienne (> 5 min)
        let old_date = "Mon, 01 Jan 2020 00:00:00 GMT";
        // La verification devrait rejeter
    }
}
```

```rust
// src/api/middleware/auth.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_requires_valid_secret() {
        // Token signe avec mauvais secret
        let bad_token = create_token_with_secret("wrong-secret");
        let result = validate_token(&bad_token, "correct-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_rejects_expired() {
        let expired_token = create_expired_token();
        let result = validate_token(&expired_token, "secret");
        assert!(result.is_err());
    }
}
```

---

## Integration CI/CD Complete

```yaml
# .github/workflows/security.yml
name: Security Pipeline

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  # SAST - Analyse statique
  sast:
    name: Static Analysis
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Format check
        run: cargo fmt --check

  # SCA - Vulnerabilites dependances
  sca:
    name: Dependency Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Audit
        run: cargo audit --deny warnings

      - name: Install cargo-deny
        run: cargo install cargo-deny

      - name: Check licenses
        run: cargo deny check licenses
        continue-on-error: true

  # Tests unitaires securite
  unit-tests:
    name: Security Unit Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run tests
        run: cargo test --all-features
        env:
          JWT_SECRET: "test-secret-32-chars-minimum-here"
          DATABASE_URL: "postgres://test:test@localhost/test"

  # DAST - Sur staging uniquement
  dast:
    name: Dynamic Analysis
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    needs: [sast, sca, unit-tests]
    steps:
      - name: ZAP Scan
        uses: zaproxy/action-api-scan@v0.7.0
        with:
          target: ${{ secrets.STAGING_URL }}
          rules_file_name: 'zap-rules.tsv'
          fail_action: false  # Alertes seulement
```

---

## Matrice de Couverture

| Exigence | SAST | DAST | SCA | Unit |
|----------|------|------|-----|------|
| AUTH-001 | X | - | - | X |
| AUTH-002 | - | X | - | X |
| AUTH-003 | X | - | - | X |
| AUTH-004 | - | X | - | X |
| AUTHZ-001 | - | X | - | X |
| AUTHZ-002 | - | X | - | X |
| VAL-001 | X | X | - | X |
| CRYPTO-001 | X | - | - | X |
| CRYPTO-003 | - | X | - | - |

---

## Metriques et Seuils

### KPIs

| Metrique | Seuil Bloquant | Cible |
|----------|----------------|-------|
| Vulns SAST Critical | 0 | 0 |
| Vulns SAST High | 0 | 0 |
| Vulns SCA CVSS >= 7.0 | 0 | 0 |
| Couverture tests securite | 60% | 80% |
| Temps scan DAST | < 60min | < 30min |

### Reporting

```bash
# Generer rapport securite
cargo audit --json > audit-report.json
cargo clippy --message-format=json > clippy-report.json
```

---

## Prochaine Etape

1. Configurer les workflows CI/CD ci-dessus
2. Executer premier scan baseline
3. Remedier les alertes critiques

```bash
# Lancer l'audit initial
cargo audit

# Lancer les tests
cargo test

# Executer clippy
cargo clippy -- -D warnings
```
