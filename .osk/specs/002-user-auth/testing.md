# Stratégie de Tests de Sécurité - User Authentication

> Généré par `/osk-specify 002-user-auth` le 2025-12-26
> Principe IV (Security Testing) - OWASP Testing Guide

## Résumé Exécutif

| Métrique | Valeur |
|----------|--------|
| Tests SAST | cargo clippy + custom lints |
| Tests SCA | cargo audit (CVSS ≥ 7) |
| Tests unitaires | 15 cas critiques |
| Tests intégration | 22 scénarios |
| Tests sécurité | 12 tests spécifiques |
| Couverture cible | 80% services auth |

---

## SAST - Static Application Security Testing

### Outil: cargo clippy + deny.toml

**Intégration CI**:
```yaml
# .github/workflows/security.yml
name: Security Checks

on: [push, pull_request]

jobs:
  sast:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - name: Run Clippy
        run: cargo clippy --all-features -- -D warnings
```

**Règles custom** (`.cargo/config.toml`):
```toml
[build]
rustflags = [
    "-W", "clippy::unwrap_used",      # Interdire unwrap en prod
    "-W", "clippy::expect_used",      # Préférer proper error handling
    "-W", "clippy::panic",            # Interdire panic en prod
    "-D", "unsafe_code",              # Interdire unsafe
]
```

**Seuils**:
- 0 warning clippy autorisé
- 0 unsafe autorisé dans code auth

---

## SCA - Software Composition Analysis

### Outil: cargo audit

**Intégration CI**:
```yaml
  sca:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install cargo-audit
        run: cargo install cargo-audit
      - name: Run Audit
        run: cargo audit --deny warnings
```

**Politique**:
- CVSS ≥ 7 : Build bloqué
- CVSS ≥ 4 : Warning, review requis
- Mise à jour automatique via Dependabot

**deny.toml**:
```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC"]
```

---

## Tests Unitaires

### Couverture Cible

| Module | Couverture |
|--------|------------|
| services/password.rs | 90% |
| services/totp.rs | 90% |
| services/session.rs | 85% |
| services/auth.rs | 80% |
| Total | 80% |

### Cas de Test Critiques

```rust
// tests/unit/password_test.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_strength_all_requirements() {
        let strength = validate_password_strength("SecurePass123");
        assert!(strength.has_min_length);
        assert!(strength.has_uppercase);
        assert!(strength.has_lowercase);
        assert!(strength.has_number);
    }

    #[test]
    fn test_password_too_short() {
        let strength = validate_password_strength("Short1A");
        assert!(!strength.has_min_length);
        assert!(!strength.is_valid());
    }

    #[test]
    fn test_password_no_uppercase() {
        let strength = validate_password_strength("lowercaseonly123");
        assert!(!strength.has_uppercase);
    }

    #[test]
    fn test_argon2_hash_format() {
        let hash = hash_password("TestPassword123").unwrap();
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_argon2_verify_correct() {
        let hash = hash_password("TestPassword123").unwrap();
        assert!(verify_password("TestPassword123", &hash).unwrap());
    }

    #[test]
    fn test_argon2_verify_incorrect() {
        let hash = hash_password("TestPassword123").unwrap();
        assert!(!verify_password("WrongPassword", &hash).unwrap());
    }
}
```

```rust
// tests/unit/session_test.rs

#[test]
fn test_session_token_length() {
    let token = generate_session_token();
    assert_eq!(token.len(), 64); // 256 bits = 32 bytes = 64 hex chars
}

#[test]
fn test_session_token_uniqueness() {
    let tokens: HashSet<_> = (0..1000).map(|_| generate_session_token()).collect();
    assert_eq!(tokens.len(), 1000); // All unique
}

#[test]
fn test_session_cookie_flags() {
    let cookie = create_session_cookie("token", Utc::now() + Duration::days(7));
    let cookie_str = cookie.to_str().unwrap();
    assert!(cookie_str.contains("HttpOnly"));
    assert!(cookie_str.contains("Secure"));
    assert!(cookie_str.contains("SameSite=Strict"));
}
```

```rust
// tests/unit/totp_test.rs

#[test]
fn test_totp_secret_generation() {
    let secret = generate_totp_secret();
    assert_eq!(secret.len(), 32); // Base32 encoded
}

#[test]
fn test_totp_verification_valid_code() {
    let secret = generate_totp_secret();
    let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret.as_bytes().to_vec(), None, "test".to_string()).unwrap();
    let code = totp.generate_current().unwrap();
    assert!(totp.check_current(&code).unwrap());
}

#[test]
fn test_recovery_code_format() {
    let codes = generate_recovery_codes(8);
    assert_eq!(codes.len(), 8);
    for code in &codes {
        assert_eq!(code.len(), 10); // Format: XXXXX-XXXXX
        assert!(code.chars().all(|c| c.is_alphanumeric() || c == '-'));
    }
}
```

---

## Tests d'Intégration

### Structure

```
tests/
├── integration/
│   ├── registration_test.rs    # US1: Registration flow
│   ├── login_test.rs           # US2: Login flow
│   ├── password_reset_test.rs  # US3: Password recovery
│   ├── session_test.rs         # US4: Session management
│   ├── totp_test.rs           # US4: 2FA flow
│   └── deletion_test.rs       # US5: Account deletion
└── security/
    ├── rate_limit_test.rs     # Rate limiting verification
    ├── enumeration_test.rs    # User enumeration prevention
    └── timing_test.rs         # Timing attack resistance
```

### Scénarios par User Story

#### US1 - Registration (7 tests)

```rust
// tests/integration/registration_test.rs

#[tokio::test]
async fn test_successful_registration() {
    let app = create_test_app().await;

    let response = app.post("/api/auth/register")
        .json(&json!({
            "email": "newuser@example.com",
            "username": "newuser",
            "password": "SecurePass123",
            "display_name": "New User"
        }))
        .await;

    assert_eq!(response.status(), 201);
    // Verify email sent (mock)
    assert!(app.email_mock.sent_to("newuser@example.com"));
}

#[tokio::test]
async fn test_registration_duplicate_email() {
    // Pre-create user
    create_user("existing@example.com").await;

    let response = app.post("/api/auth/register")
        .json(&json!({
            "email": "existing@example.com",
            "username": "newuser",
            "password": "SecurePass123"
        }))
        .await;

    // Same response as success (no enumeration)
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await;
    assert!(body["message"].as_str().unwrap().contains("verification"));
}

#[tokio::test]
async fn test_registration_weak_password() {
    let response = app.post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "username": "testuser",
            "password": "weak"  // Too short, no requirements
        }))
        .await;

    assert_eq!(response.status(), 400);
    let body: Value = response.json().await;
    assert!(body["fields"]["password"].is_array());
}

#[tokio::test]
async fn test_registration_reserved_username() {
    let response = app.post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "username": "admin",  // Reserved
            "password": "SecurePass123"
        }))
        .await;

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_email_confirmation_success() {
    let token = register_and_get_token("confirm@example.com").await;

    let response = app.get(&format!("/api/auth/confirm-email/{}", token)).await;

    assert_eq!(response.status(), 200);
    // Verify user is now verified
    let user = get_user_by_email("confirm@example.com").await;
    assert!(user.email_verified);
}

#[tokio::test]
async fn test_email_confirmation_expired() {
    let token = register_and_get_token("expired@example.com").await;
    // Expire token manually
    expire_token(&token).await;

    let response = app.get(&format!("/api/auth/confirm-email/{}", token)).await;

    assert_eq!(response.status(), 400);
    let body: Value = response.json().await;
    assert!(body["error"].as_str().unwrap().contains("expired"));
}

#[tokio::test]
async fn test_email_confirmation_already_used() {
    let token = register_and_get_token("used@example.com").await;
    app.get(&format!("/api/auth/confirm-email/{}", token)).await;

    // Try again
    let response = app.get(&format!("/api/auth/confirm-email/{}", token)).await;

    assert_eq!(response.status(), 400);
}
```

#### US2 - Login (6 tests)

```rust
// tests/integration/login_test.rs

#[tokio::test]
async fn test_successful_login() {
    create_verified_user("login@example.com", "SecurePass123").await;

    let response = app.post("/api/auth/login")
        .json(&json!({
            "email": "login@example.com",
            "password": "SecurePass123"
        }))
        .await;

    assert_eq!(response.status(), 200);
    assert!(response.headers().contains_key("set-cookie"));
    let cookie = response.headers().get("set-cookie").unwrap().to_str().unwrap();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));
}

#[tokio::test]
async fn test_login_wrong_password() {
    create_verified_user("wrong@example.com", "SecurePass123").await;

    let response = app.post("/api/auth/login")
        .json(&json!({
            "email": "wrong@example.com",
            "password": "WrongPassword"
        }))
        .await;

    assert_eq!(response.status(), 401);
    let body: Value = response.json().await;
    assert_eq!(body["error"], "Invalid credentials"); // Generic message
}

#[tokio::test]
async fn test_login_unknown_user() {
    let response = app.post("/api/auth/login")
        .json(&json!({
            "email": "unknown@example.com",
            "password": "AnyPassword123"
        }))
        .await;

    assert_eq!(response.status(), 401);
    let body: Value = response.json().await;
    assert_eq!(body["error"], "Invalid credentials"); // Same message as wrong password
}

#[tokio::test]
async fn test_login_account_lockout() {
    create_verified_user("lockout@example.com", "SecurePass123").await;

    // 5 failed attempts
    for _ in 0..5 {
        app.post("/api/auth/login")
            .json(&json!({
                "email": "lockout@example.com",
                "password": "WrongPassword"
            }))
            .await;
    }

    // 6th attempt
    let response = app.post("/api/auth/login")
        .json(&json!({
            "email": "lockout@example.com",
            "password": "SecurePass123"  // Correct password
        }))
        .await;

    assert_eq!(response.status(), 403);
    let body: Value = response.json().await;
    assert!(body["locked_until"].is_string());
}

#[tokio::test]
async fn test_login_unverified_email() {
    create_unverified_user("unverified@example.com", "SecurePass123").await;

    let response = app.post("/api/auth/login")
        .json(&json!({
            "email": "unverified@example.com",
            "password": "SecurePass123"
        }))
        .await;

    assert_eq!(response.status(), 403);
    let body: Value = response.json().await;
    assert!(body["error"].as_str().unwrap().contains("verify"));
}

#[tokio::test]
async fn test_logout() {
    let session = login_and_get_session("logout@example.com").await;

    let response = app.post("/api/auth/logout")
        .header("Cookie", format!("session={}", session))
        .await;

    assert_eq!(response.status(), 200);

    // Verify session invalid
    let protected = app.get("/api/account/profile")
        .header("Cookie", format!("session={}", session))
        .await;
    assert_eq!(protected.status(), 401);
}
```

#### US3 - Password Reset (5 tests)

```rust
// tests/integration/password_reset_test.rs

#[tokio::test]
async fn test_forgot_password_existing_email() {
    create_verified_user("forgot@example.com", "OldPassword123").await;

    let response = app.post("/api/auth/forgot-password")
        .json(&json!({ "email": "forgot@example.com" }))
        .await;

    assert_eq!(response.status(), 200);
    assert!(app.email_mock.sent_to("forgot@example.com"));
}

#[tokio::test]
async fn test_forgot_password_unknown_email() {
    let response = app.post("/api/auth/forgot-password")
        .json(&json!({ "email": "unknown@example.com" }))
        .await;

    // Same response (no enumeration)
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await;
    assert!(body["message"].as_str().unwrap().contains("instructions"));
}

#[tokio::test]
async fn test_reset_password_success() {
    let token = request_reset_token("reset@example.com").await;

    let response = app.post("/api/auth/reset-password")
        .json(&json!({
            "token": token,
            "new_password": "NewSecurePass123"
        }))
        .await;

    assert_eq!(response.status(), 200);

    // Verify new password works
    let login = app.post("/api/auth/login")
        .json(&json!({
            "email": "reset@example.com",
            "password": "NewSecurePass123"
        }))
        .await;
    assert_eq!(login.status(), 200);
}

#[tokio::test]
async fn test_reset_password_token_reuse() {
    let token = request_reset_token("reuse@example.com").await;
    app.post("/api/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "FirstPass123" }))
        .await;

    // Try again
    let response = app.post("/api/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "SecondPass123" }))
        .await;

    assert_eq!(response.status(), 400); // Token already used
}

#[tokio::test]
async fn test_reset_password_expired_token() {
    let token = request_reset_token("expire@example.com").await;
    expire_reset_token(&token).await; // Set expires_at to past

    let response = app.post("/api/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "NewPass123" }))
        .await;

    assert_eq!(response.status(), 400);
}
```

---

## Tests de Sécurité Spécifiques

### Rate Limiting

```rust
// tests/security/rate_limit_test.rs

#[tokio::test]
async fn test_login_rate_limit_by_ip() {
    let app = create_test_app().await;

    // 10 requests should succeed
    for i in 0..10 {
        let response = app.post("/api/auth/login")
            .json(&json!({
                "email": format!("user{}@example.com", i),
                "password": "AnyPassword"
            }))
            .await;
        assert_ne!(response.status(), 429);
    }

    // 11th should be rate limited
    let response = app.post("/api/auth/login")
        .json(&json!({
            "email": "user11@example.com",
            "password": "AnyPassword"
        }))
        .await;

    assert_eq!(response.status(), 429);
}

#[tokio::test]
async fn test_registration_rate_limit() {
    for i in 0..5 {
        app.post("/api/auth/register")
            .json(&json!({
                "email": format!("rate{}@example.com", i),
                "username": format!("rate{}", i),
                "password": "SecurePass123"
            }))
            .await;
    }

    let response = app.post("/api/auth/register")
        .json(&json!({
            "email": "rate6@example.com",
            "username": "rate6",
            "password": "SecurePass123"
        }))
        .await;

    assert_eq!(response.status(), 429);
}

#[tokio::test]
async fn test_password_reset_rate_limit_per_email() {
    create_verified_user("ratelimit@example.com", "Pass123").await;

    // First request OK
    let r1 = app.post("/api/auth/forgot-password")
        .json(&json!({ "email": "ratelimit@example.com" }))
        .await;
    assert_eq!(r1.status(), 200);

    // Second request within 5 min
    let r2 = app.post("/api/auth/forgot-password")
        .json(&json!({ "email": "ratelimit@example.com" }))
        .await;
    assert_eq!(r2.status(), 429);
}
```

### User Enumeration Prevention

```rust
// tests/security/enumeration_test.rs

#[tokio::test]
async fn test_login_no_user_enumeration() {
    create_verified_user("exists@example.com", "Pass123").await;

    let response_exists = app.post("/api/auth/login")
        .json(&json!({ "email": "exists@example.com", "password": "WrongPass" }))
        .await;

    let response_not_exists = app.post("/api/auth/login")
        .json(&json!({ "email": "notexists@example.com", "password": "AnyPass" }))
        .await;

    // Same status and message
    assert_eq!(response_exists.status(), response_not_exists.status());
    let body1: Value = response_exists.json().await;
    let body2: Value = response_not_exists.json().await;
    assert_eq!(body1["error"], body2["error"]);
}

#[tokio::test]
async fn test_registration_no_email_enumeration() {
    create_verified_user("taken@example.com", "Pass123").await;

    let response_new = app.post("/api/auth/register")
        .json(&json!({
            "email": "new@example.com",
            "username": "newuser",
            "password": "SecurePass123"
        }))
        .await;

    let response_taken = app.post("/api/auth/register")
        .json(&json!({
            "email": "taken@example.com",
            "username": "takenuser",
            "password": "SecurePass123"
        }))
        .await;

    // Same status (201) and similar message
    assert_eq!(response_new.status(), 201);
    assert_eq!(response_taken.status(), 201);
}

#[tokio::test]
async fn test_reset_no_email_enumeration() {
    create_verified_user("resetexists@example.com", "Pass123").await;

    let r1 = app.post("/api/auth/forgot-password")
        .json(&json!({ "email": "resetexists@example.com" }))
        .await;

    let r2 = app.post("/api/auth/forgot-password")
        .json(&json!({ "email": "resetnotexists@example.com" }))
        .await;

    assert_eq!(r1.status(), r2.status());
    let b1: Value = r1.json().await;
    let b2: Value = r2.json().await;
    assert_eq!(b1["message"], b2["message"]);
}
```

### Timing Attack Resistance

```rust
// tests/security/timing_test.rs
use std::time::Instant;

#[tokio::test]
async fn test_login_constant_time() {
    create_verified_user("timing@example.com", "SecurePass123").await;

    let mut times_existing = vec![];
    let mut times_not_existing = vec![];

    for _ in 0..100 {
        let start = Instant::now();
        app.post("/api/auth/login")
            .json(&json!({ "email": "timing@example.com", "password": "WrongPass" }))
            .await;
        times_existing.push(start.elapsed());

        let start = Instant::now();
        app.post("/api/auth/login")
            .json(&json!({ "email": "notexist@example.com", "password": "AnyPass" }))
            .await;
        times_not_existing.push(start.elapsed());
    }

    let avg_existing = average(&times_existing);
    let avg_not_existing = average(&times_not_existing);

    // Difference should be < 10ms (accounting for network variance)
    let diff = (avg_existing.as_millis() as i64 - avg_not_existing.as_millis() as i64).abs();
    assert!(diff < 10, "Timing difference: {}ms", diff);
}
```

---

## Exécution CI/CD

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
          POSTGRES_DB: oppskrift_test
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run migrations
        run: cargo sqlx migrate run
        env:
          DATABASE_URL: postgres://test:test@localhost/oppskrift_test

      - name: Run tests
        run: cargo test --all-features
        env:
          DATABASE_URL: postgres://test:test@localhost/oppskrift_test
          JWT_SECRET: test-secret-32-chars-minimum-here
          TOTP_ENCRYPTION_KEY: test-totp-key-32-chars-here

      - name: Run security tests
        run: cargo test --test security_tests
```

---

## Métriques et Seuils

| Métrique | Seuil | Action si échoué |
|----------|-------|------------------|
| Couverture tests | ≥ 80% | Block merge |
| Tests sécurité | 100% pass | Block merge |
| Vulnérabilités SCA | CVSS < 7 | Block merge |
| Warnings clippy | 0 | Block merge |
| Timing variance | < 10ms | Warning |

---

## Prochaine Étape

→ Exécuter `/speckit.implement` pour implémenter les tests
→ Configurer CI/CD avec les workflows ci-dessus
→ Planifier test de pénétration manuel post-implémentation
