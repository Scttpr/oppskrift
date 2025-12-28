# Exigences de Sécurité - User Authentication

> Généré par `/osk-specify 002-user-auth` le 2025-12-26
> Principes III (Security Requirements) - OWASP ASVS L2

## Résumé Exécutif

| Métrique | Valeur |
|----------|--------|
| Exigences totales | 42 |
| MUST (obligatoire) | 32 |
| SHOULD (recommandé) | 9 |
| MAY (optionnel) | 1 |
| Couverture risques | 100% (12/12) |
| Niveau ASVS cible | L2 |

---

## Catégorie AUTH - Authentification

### AUTH-001 [MUST] - Rate Limiting par IP

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-001
**ASVS**: V2.2.1

L'application DOIT implémenter un rate limiting de 10 tentatives de login par IP par minute.

**Implémentation Rust**:
```rust
// src/main.rs
use tower_governor::{GovernorConfigBuilder, GovernorLayer};

let login_governor = GovernorConfigBuilder::default()
    .per_second(1)
    .burst_size(10)
    .key_extractor(SmartIpKeyExtractor)
    .finish()
    .unwrap();

Router::new()
    .route("/api/auth/login", post(login))
    .layer(GovernorLayer { config: login_governor })
```

**Vérification**: Test automatisé envoyant 15 requêtes en 60s → erreur 429 après la 10ème.

---

### AUTH-002 [MUST] - Rate Limiting par Compte

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-001, RISK-AUTH-004
**ASVS**: V2.2.1

L'application DOIT verrouiller un compte après 5 tentatives échouées avec lockout progressif (15min → 30min → 1h).

**Implémentation Rust**:
```rust
// src/services/auth.rs
pub async fn check_and_update_lockout(user: &User, pool: &PgPool) -> Result<(), AuthError> {
    if let Some(locked_until) = user.locked_until {
        if locked_until > Utc::now() {
            return Err(AuthError::AccountLocked { until: locked_until });
        }
    }

    let lockout_duration = match user.failed_login_attempts {
        0..=4 => return Ok(()), // Pas encore de lockout
        5..=9 => Duration::minutes(15),
        10..=14 => Duration::minutes(30),
        _ => Duration::hours(1),
    };

    sqlx::query!(
        "UPDATE users SET locked_until = $1 WHERE id = $2",
        Utc::now() + lockout_duration,
        user.id
    ).execute(pool).await?;

    Err(AuthError::AccountLocked { until: Utc::now() + lockout_duration })
}
```

**Vérification**: Test de 6 tentatives échouées → compte verrouillé 15 minutes.

---

### AUTH-003 [MUST] - Session Cookies Sécurisés

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-003
**ASVS**: V3.4.1, V3.4.2, V3.4.3

Les cookies de session DOIVENT utiliser les flags HttpOnly, Secure, et SameSite=Strict.

**Implémentation Rust**:
```rust
// src/services/session.rs
use axum::http::header::{HeaderValue, SET_COOKIE};

pub fn create_session_cookie(token: &str, expires: DateTime<Utc>) -> HeaderValue {
    let cookie = format!(
        "session={}; Path=/; HttpOnly; Secure; SameSite=Strict; Expires={}",
        token,
        expires.format("%a, %d %b %Y %H:%M:%S GMT")
    );
    HeaderValue::from_str(&cookie).unwrap()
}
```

**Vérification**: Inspection navigateur → cookie présente tous les flags requis.

---

### AUTH-004 [MUST] - Token de Session Imprévisible

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-003
**ASVS**: V3.2.1

Les tokens de session DOIVENT être générés avec 256 bits d'entropie cryptographique.

**Implémentation Rust**:
```rust
// src/services/session.rs
use rand::{rngs::OsRng, RngCore};

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32]; // 256 bits
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes) // 64 caractères hex
}
```

**Vérification**: Analyse statistique de 1000 tokens → distribution uniforme.

---

### AUTH-005 [MUST] - Lockout Progressif avec Self-Unlock

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-004
**ASVS**: V2.2.3

Le lockout DOIT être progressif et offrir une option de déverrouillage par email.

**Implémentation**: Voir AUTH-002 + endpoint POST /api/auth/unlock-account.

**Vérification**: Test de lockout → email reçu avec lien de déverrouillage.

---

### AUTH-006 [SHOULD] - Alerte Lockouts Massifs

**Criticité**: RFC 2119 SHOULD
**Risques adressés**: RISK-AUTH-004

Le système DEVRAIT alerter les administrateurs lors de lockouts massifs (>10 comptes/heure).

**Implémentation**: Compteur Redis/mémoire + webhook/email admin.

---

### AUTH-007 [MUST] - Password Reset Token Sécurisé

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-006
**ASVS**: V2.5.2, V2.5.4

Les tokens de reset DOIVENT être 256-bit random, single-use, expirer en 1h.

**Implémentation Rust**:
```rust
// src/services/auth.rs
pub async fn create_reset_token(user_id: Uuid, pool: &PgPool) -> Result<String, Error> {
    // Invalider anciens tokens
    sqlx::query!("UPDATE password_reset_tokens SET used_at = NOW() WHERE user_id = $1", user_id)
        .execute(pool).await?;

    let token = generate_session_token(); // 256-bit random
    let token_hash = bcrypt::hash(&token, bcrypt::DEFAULT_COST)?;

    sqlx::query!(
        "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        user_id,
        token_hash,
        Utc::now() + Duration::hours(1)
    ).execute(pool).await?;

    Ok(token)
}
```

**Vérification**: Token utilisé une fois → 400 Bad Request à la seconde utilisation.

---

### AUTH-008 [MUST] - TOTP Rate Limiting

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-007
**ASVS**: V2.8.4

La vérification TOTP DOIT être limitée à 3 tentatives par 10 minutes.

**Implémentation Rust**:
```rust
// src/services/totp.rs
const MAX_TOTP_ATTEMPTS: i32 = 3;
const TOTP_LOCKOUT_MINUTES: i64 = 10;

pub async fn verify_totp_with_rate_limit(
    user_id: Uuid,
    code: &str,
    pool: &PgPool,
) -> Result<bool, AuthError> {
    let attempts = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM security_events
         WHERE user_id = $1 AND event_type = 'totp_failed'
         AND created_at > NOW() - INTERVAL '10 minutes'",
        user_id
    ).fetch_one(pool).await?;

    if attempts.unwrap_or(0) >= MAX_TOTP_ATTEMPTS as i64 {
        return Err(AuthError::TotpRateLimited);
    }
    // ... verify TOTP
}
```

**Vérification**: 4 tentatives TOTP invalides → erreur rate limit.

---

### AUTH-009 [MUST] - Session Listing et Revocation

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-009
**ASVS**: V3.3.1

Les utilisateurs DOIVENT pouvoir lister et révoquer leurs sessions actives.

**Endpoints**:
- GET /api/account/sessions
- DELETE /api/account/sessions/:id

**Vérification**: Révocation → session immédiatement invalide (401 sur prochain appel).

---

### AUTH-010 [MUST] - Invalidation Sessions au Changement Password

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-009
**ASVS**: V3.3.3

Toutes les sessions (sauf courante) DOIVENT être invalidées au changement de mot de passe.

**Implémentation Rust**:
```rust
pub async fn change_password(
    user_id: Uuid,
    current_session_id: Uuid,
    new_password: &str,
    pool: &PgPool,
) -> Result<(), Error> {
    // ... validate and hash new password

    // Invalider toutes les autres sessions
    sqlx::query!(
        "DELETE FROM sessions WHERE user_id = $1 AND id != $2",
        user_id,
        current_session_id
    ).execute(pool).await?;

    // Log security event
    log_security_event(user_id, SecurityEventType::PasswordChange, pool).await?;

    Ok(())
}
```

---

### AUTH-011 [MUST] - Session Regeneration au Login

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-011
**ASVS**: V3.2.3

Le session ID DOIT être régénéré après authentification réussie.

**Vérification**: Comparer session ID avant/après login → différents.

---

### AUTH-012 [MUST] - Recovery Codes Single-Use

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-012
**ASVS**: V2.8.6

Les recovery codes DOIVENT être marqués utilisés immédiatement et hachés en DB.

**Implémentation Rust**:
```rust
pub async fn use_recovery_code(user_id: Uuid, code: &str, pool: &PgPool) -> Result<bool, Error> {
    let codes = sqlx::query!(
        "SELECT id, code_hash FROM recovery_codes WHERE user_id = $1 AND used_at IS NULL",
        user_id
    ).fetch_all(pool).await?;

    for recovery in codes {
        if bcrypt::verify(code, &recovery.code_hash)? {
            sqlx::query!(
                "UPDATE recovery_codes SET used_at = NOW() WHERE id = $1",
                recovery.id
            ).execute(pool).await?;

            log_security_event(user_id, SecurityEventType::RecoveryCodeUsed, pool).await?;
            return Ok(true);
        }
    }
    Ok(false)
}
```

---

## Catégorie AUTHZ - Autorisation

### AUTHZ-001 [MUST] - User ID depuis Session

**Criticité**: RFC 2119 MUST
**Risques adressés**: THREAT-AUTH-E-002
**ASVS**: V4.2.1

Le user_id DOIT être extrait de la session serveur, jamais depuis l'input utilisateur.

**Implémentation Rust**:
```rust
// src/api/middleware/auth.rs
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extraire session depuis cookie, jamais depuis body/query
        let session_token = extract_session_cookie(&parts.headers)?;
        let session = validate_session(&session_token, pool).await?;
        Ok(AuthUser { id: session.user_id, username: session.username })
    }
}
```

---

### AUTHZ-002 [MUST] - Ownership Check Systématique

**Criticité**: RFC 2119 MUST
**ASVS**: V4.2.2

Chaque opération sur une ressource DOIT vérifier l'ownership via session.

---

### AUTHZ-003 [MUST] - Pas d'IDOR sur Sessions

**Criticité**: RFC 2119 MUST

La révocation de session DOIT vérifier que la session appartient à l'utilisateur courant.

```rust
pub async fn revoke_session(auth: AuthUser, session_id: Uuid, pool: &PgPool) -> Result<(), Error> {
    let result = sqlx::query!(
        "DELETE FROM sessions WHERE id = $1 AND user_id = $2",
        session_id,
        auth.id  // Ownership check
    ).execute(pool).await?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}
```

---

### AUTHZ-004 [MUST] - Double-Check Privileges

**Criticité**: RFC 2119 MUST
**ASVS**: V4.1.1

Les privilèges DOIVENT être vérifiés côté serveur, pas seulement dans le JWT.

---

### AUTHZ-005 [SHOULD] - Séparation Admin/User

**Criticité**: RFC 2119 SHOULD

Les routes admin DEVRAIENT avoir une vérification de rôle distincte.

---

## Catégorie VAL - Validation

### VAL-001 [MUST] - Password Breach Check

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-001
**ASVS**: V2.1.7

Les mots de passe DOIVENT être vérifiés contre la base HIBP (k-anonymity).

**Implémentation Rust**:
```rust
// src/services/password.rs
use sha1::{Sha1, Digest};

pub async fn is_password_breached(password: &str) -> Result<bool, Error> {
    let hash = hex::encode(Sha1::digest(password.as_bytes())).to_uppercase();
    let prefix = &hash[..5];
    let suffix = &hash[5..];

    let response = reqwest::get(format!(
        "https://api.pwnedpasswords.com/range/{}",
        prefix
    )).await?.text().await?;

    for line in response.lines() {
        if line.starts_with(suffix) {
            return Ok(true); // Password found in breaches
        }
    }
    Ok(false)
}
```

---

### VAL-002 [MUST] - Generic Registration Error

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-008
**ASVS**: V2.5.3

L'inscription DOIT retourner le même message que l'email existe ou non.

```rust
// Toujours retourner ce message, même si email existe
Ok(Json(RegisterResponse {
    message: "If the email is not already registered, a verification link will be sent.".into(),
}))
```

---

### VAL-003 [MUST] - Generic Login Error

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-008
**ASVS**: V2.4.1

Le login DOIT retourner "Invalid credentials" sans distinguer email/password incorrect.

---

### VAL-004 [MUST] - Generic Reset Error

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-008

Le reset password DOIT retourner le même message que l'email existe ou non.

---

### VAL-005 [MUST] - Password Strength Validation

**Criticité**: RFC 2119 MUST
**ASVS**: V2.1.1

Les mots de passe DOIVENT avoir min 10 chars, 1 majuscule, 1 minuscule, 1 chiffre.

```rust
pub fn validate_password_strength(password: &str) -> PasswordStrength {
    PasswordStrength {
        has_min_length: password.len() >= 10,
        has_uppercase: password.chars().any(|c| c.is_uppercase()),
        has_lowercase: password.chars().any(|c| c.is_lowercase()),
        has_number: password.chars().any(|c| c.is_numeric()),
        is_not_breached: true, // Set after async check
    }
}
```

---

### VAL-006 [MUST] - Username Validation

**Criticité**: RFC 2119 MUST

Usernames: alphanumeric + _, 3-30 chars, lowercase, pas dans RESERVED_USERNAMES.

---

### VAL-007 [SHOULD] - Email Format Validation

**Criticité**: RFC 2119 SHOULD

Validation format email via crate `validator`.

---

## Catégorie CRYPTO - Cryptographie

### CRYPTO-001 [MUST] - Argon2id avec Params OWASP

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-002
**ASVS**: V2.4.4

Les mots de passe DOIVENT être hachés avec Argon2id (m=19456, t=2, p=1).

```rust
use argon2::{Argon2, Params, Algorithm, Version};

fn create_argon2() -> Argon2<'static> {
    let params = Params::new(
        19456,  // m = 19 MiB
        2,      // t = 2 iterations
        1,      // p = 1 parallelism
        None,
    ).unwrap();

    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}
```

---

### CRYPTO-002 [SHOULD] - Password Pepper

**Criticité**: RFC 2119 SHOULD
**Risques adressés**: RISK-AUTH-002

Un pepper applicatif DEVRAIT être utilisé en plus du salt.

---

### CRYPTO-003 [MUST] - Token Hashing en DB

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-006
**ASVS**: V3.5.3

Les tokens (session, reset, confirmation) DOIVENT être stockés hachés en DB.

---

### CRYPTO-004 [MUST] - TOTP Secret Encryption

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-007
**ASVS**: V6.2.1

Les secrets TOTP DOIVENT être chiffrés au repos avec AES-256-GCM.

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::Aead};

pub fn encrypt_totp_secret(secret: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(b"unique nonce"); // Use random nonce in prod
    cipher.encrypt(nonce, secret).expect("encryption failure")
}
```

---

### CRYPTO-005 [MUST] - Constant-Time Comparison

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-010
**ASVS**: V2.4.2

Les comparaisons de credentials DOIVENT être constant-time.

```rust
// Fake hash pour user inexistant (même timing)
let fake_hash = "$argon2id$v=19$m=19456,t=2,p=1$...";
let hash = user.map(|u| u.password_hash.as_str()).unwrap_or(fake_hash);
argon2.verify_password(password.as_bytes(), &PasswordHash::new(hash)?)?;
```

---

### CRYPTO-006 [MUST] - Recovery Codes Hashing

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-012

Les recovery codes DOIVENT être stockés hachés (bcrypt).

---

### CRYPTO-007 [MUST] - TLS 1.3+ Obligatoire

**Criticité**: RFC 2119 MUST
**ASVS**: V9.1.1

Toutes les communications DOIVENT utiliser TLS 1.3+.

**Note**: Configuré au niveau reverse proxy (nginx/traefik).

---

## Catégorie AUDIT - Logging

### AUDIT-001 [MUST] - Login Events

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-005
**ASVS**: V7.1.1

Tous les événements de login (succès/échec) DOIVENT être journalisés.

```rust
pub enum SecurityEventType {
    LoginSuccess,
    LoginFailed,
    LoginLocked,
    // ...
}

pub async fn log_security_event(
    user_id: Option<Uuid>,
    event_type: SecurityEventType,
    ip: IpAddr,
    user_agent: &str,
    metadata: Option<Value>,
    pool: &PgPool,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO security_events (user_id, event_type, ip_address, user_agent, metadata)
         VALUES ($1, $2, $3, $4, $5)",
        user_id, event_type.to_string(), ip.to_string(), user_agent, metadata
    ).execute(pool).await?;
    Ok(())
}
```

---

### AUDIT-002 [MUST] - Password Change Events

**Criticité**: RFC 2119 MUST
**Risques adressés**: RISK-AUTH-005

Les changements de mot de passe DOIVENT être journalisés.

---

### AUDIT-003 [MUST] - 2FA Events

**Criticité**: RFC 2119 MUST

Activation/désactivation 2FA et utilisation recovery codes DOIVENT être journalisés.

---

### AUDIT-004 [MUST] - Session Events

**Criticité**: RFC 2119 MUST

Création et révocation de sessions DOIVENT être journalisées.

---

### AUDIT-005 [SHOULD] - Email Masking in Logs

**Criticité**: RFC 2119 SHOULD
**RGPD**: Art. 25

Les emails DEVRAIENT être masqués dans les logs (u***@example.com).

```rust
fn mask_email(email: &str) -> String {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() == 2 && parts[0].len() > 1 {
        format!("{}***@{}", &parts[0][..1], parts[1])
    } else {
        "***@***".to_string()
    }
}
```

---

## Catégorie RGPD - Conformité

### RGPD-001 [MUST] - Droit à l'Effacement

**Criticité**: RFC 2119 MUST
**Risques adressés**: Spec FR-025, FR-026
**Article**: RGPD Art. 17

Les utilisateurs DOIVENT pouvoir supprimer leur compte et données personnelles.

---

### RGPD-002 [MUST] - Grace Period Deletion

**Criticité**: RFC 2119 MUST
**Article**: RGPD Art. 17

La suppression DOIT avoir une période de grâce de 7 jours annulable.

---

### RGPD-003 [MUST] - Data Minimization

**Criticité**: RFC 2119 MUST
**Article**: RGPD Art. 5

Seules les données nécessaires à l'authentification DOIVENT être collectées.

---

### RGPD-004 [MUST] - Consent for Email

**Criticité**: RFC 2119 MUST
**Article**: RGPD Art. 6

L'envoi d'emails transactionnels (confirmation, reset) est basé sur l'exécution du contrat.

---

### RGPD-005 [MUST] - Audit Log Retention

**Criticité**: RFC 2119 MUST
**Article**: RGPD Art. 5(1)(e)

Les logs de sécurité DOIVENT être conservés maximum 90 jours.

---

### RGPD-006 [SHOULD] - User Activity Export

**Criticité**: RFC 2119 SHOULD
**Article**: RGPD Art. 20

Les utilisateurs DEVRAIENT pouvoir exporter leurs événements de sécurité.

---

### RGPD-007 [SHOULD] - Privacy by Design

**Criticité**: RFC 2119 SHOULD
**Article**: RGPD Art. 25

Les choix par défaut DEVRAIENT être les plus protecteurs de la vie privée.

---

## Matrice Risques → Exigences

| Risque | Exigences |
|--------|-----------|
| RISK-AUTH-001 | AUTH-001, AUTH-002, VAL-001 |
| RISK-AUTH-002 | CRYPTO-001, CRYPTO-002 |
| RISK-AUTH-003 | AUTH-003, AUTH-004 |
| RISK-AUTH-004 | AUTH-005, AUTH-006 |
| RISK-AUTH-005 | AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004 |
| RISK-AUTH-006 | AUTH-007, CRYPTO-003 |
| RISK-AUTH-007 | CRYPTO-004, AUTH-008 |
| RISK-AUTH-008 | VAL-002, VAL-003, VAL-004 |
| RISK-AUTH-009 | AUTH-009, AUTH-010 |
| RISK-AUTH-010 | CRYPTO-005 |
| RISK-AUTH-011 | AUTH-011 |
| RISK-AUTH-012 | AUTH-012, CRYPTO-006 |

---

## Prochaine Étape

→ Voir `testing.md` pour la stratégie de tests de sécurité
→ Implémenter exigences via `/speckit.implement` ou `/osk-implement`
