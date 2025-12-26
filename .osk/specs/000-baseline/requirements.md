# Exigences de Securite - Baseline

> Genere par `/osk-specify baseline` le 2025-12-26
> Principe III - Security by Design

## Resume

| Categorie | Exigences | MUST | SHOULD | MAY |
|-----------|-----------|------|--------|-----|
| Authentification | 6 | 4 | 2 | 0 |
| Autorisation | 4 | 3 | 1 | 0 |
| Validation | 4 | 2 | 2 | 0 |
| Chiffrement | 4 | 3 | 1 | 0 |
| **Total** | **18** | **12** | **6** | **0** |

---

## Authentification

### AUTH-001 - Configuration JWT Securisee

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | VULN-BASELINE-001 |
| Verification | `cargo test` + tentative demarrage sans JWT_SECRET |
| Statut | IMPLEMENTE |

**Description :**
Le secret JWT DOIT etre configure via variable d'environnement et valide au demarrage. Aucun fallback n'est accepte.

**Implementation :**
```rust
// src/lib/config.rs
let jwt_secret = env::var("JWT_SECRET")
    .expect("JWT_SECRET must be set");
if jwt_secret.len() < 32 {
    panic!("JWT_SECRET must be at least 32 characters");
}
```

---

### AUTH-002 - Endpoint Login avec JWT

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | THREAT-S-001 |
| Verification | Test integration endpoint /api/v1/auth/login |
| Statut | A IMPLEMENTER |

**Description :**
Un endpoint de connexion DOIT etre implemente pour authentifier les utilisateurs et generer des tokens JWT.

**Implementation requise :**
```rust
// POST /api/v1/auth/login
#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    expires_at: DateTime<Utc>,
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    // 1. Verifier credentials
    // 2. Generer JWT avec claims (sub, exp, iat)
    // 3. Logger auth.login.success
    // 4. Retourner token
}
```

---

### AUTH-003 - Hashage Mots de Passe

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | CWE-916 |
| Verification | Verification format hash en DB |
| Statut | A IMPLEMENTER |

**Description :**
Les mots de passe DOIVENT etre hashes avec Argon2id avant stockage.

**Implementation requise :**
```rust
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;

// Hashage
let salt = SaltString::generate(&mut OsRng);
let argon2 = Argon2::default();
let hash = argon2.hash_password(password.as_bytes(), &salt)?
    .to_string();

// Verification
let parsed_hash = PasswordHash::new(&stored_hash)?;
argon2.verify_password(password.as_bytes(), &parsed_hash)?;
```

---

### AUTH-004 - Verification Signatures HTTP ActivityPub

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | VULN-BASELINE-003, THREAT-S-002 |
| Verification | Tests unitaires signature + test inbox |
| Statut | IMPLEMENTE |

**Description :**
Les requetes entrantes vers les inboxes ActivityPub DOIVENT avoir leur signature HTTP verifiee.

**Implementation :**
```rust
// src/lib/activitypub/signature.rs
pub async fn verify_signature(
    signature: &HttpSignature,
    method: &str,
    path: &str,
    headers: &[(String, String)],
) -> AppResult<VerificationResult>
```

---

### AUTH-005 - Expiration Tokens JWT

| Attribut | Valeur |
|----------|--------|
| Criticite | **SHOULD** |
| Risques adresses | THREAT-S-001 |
| Verification | Verification claim exp dans token |
| Statut | A IMPLEMENTER |

**Description :**
Les tokens JWT DEVRAIENT expirer apres une duree limitee (recommande: 24h).

**Implementation :**
```rust
let claims = Claims {
    sub: user.id.to_string(),
    exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
    iat: Utc::now().timestamp() as usize,
};
```

---

### AUTH-006 - Refresh Tokens

| Attribut | Valeur |
|----------|--------|
| Criticite | **SHOULD** |
| Risques adresses | UX, Securite sessions longues |
| Verification | Endpoint /api/v1/auth/refresh |
| Statut | A IMPLEMENTER |

**Description :**
Un mecanisme de refresh token DEVRAIT etre implemente pour renouveler les sessions sans re-authentification.

---

## Autorisation

### AUTHZ-001 - Verification Owner sur Ressources

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | THREAT-E-001, THREAT-T-002 |
| Verification | Tests IDOR sur endpoints |
| Statut | IMPLEMENTE |

**Description :**
Toute modification de ressource DOIT verifier que l'utilisateur est le proprietaire.

**Implementation existante :**
```rust
// src/api/recipes.rs
if existing.author_id != auth.id {
    return Err(AppError::Forbidden("Not recipe owner".to_string()));
}
```

---

### AUTHZ-002 - Verification Visibilite Recettes

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | THREAT-I-001 |
| Verification | Test acces recette privee sans auth |
| Statut | A VERIFIER |

**Description :**
Les recettes avec visibilite != public NE DOIVENT PAS etre accessibles via ActivityPub sans autorisation.

**Implementation requise :**
```rust
// src/api/activitypub.rs - get_recipe_object
if recipe.visibility != Visibility::Public {
    return Err(StatusCode::NOT_FOUND);
}
```

---

### AUTHZ-003 - Separation Roles Admin/User

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | THREAT-E-002 |
| Verification | Schema DB + middleware |
| Statut | A IMPLEMENTER |

**Description :**
Un systeme de roles DOIT distinguer les utilisateurs normaux des administrateurs.

**Implementation requise :**
```sql
-- Migration
ALTER TABLE users ADD COLUMN role VARCHAR(20) NOT NULL DEFAULT 'user';
CREATE TYPE user_role AS ENUM ('user', 'moderator', 'admin');
```

```rust
// Middleware
pub struct RequireAdmin;

impl<S> FromRequestParts<S> for RequireAdmin {
    // Verifier auth.role == "admin"
}
```

---

### AUTHZ-004 - Rate Limiting Endpoints Sensibles

| Attribut | Valeur |
|----------|--------|
| Criticite | **SHOULD** |
| Risques adresses | THREAT-D-001, THREAT-D-002 |
| Verification | Test charge sur inbox |
| Statut | PARTIEL |

**Description :**
Les endpoints sensibles (login, inbox) DEVRAIENT avoir un rate limiting specifique.

---

## Validation

### VAL-001 - Validation Entrees API

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | CWE-20 |
| Verification | Tests avec payloads invalides |
| Statut | IMPLEMENTE |

**Description :**
Toutes les entrees API DOIVENT etre validees avec le crate `validator`.

**Implementation existante :**
```rust
#[derive(Deserialize, Validate)]
pub struct CreateRecipe {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(max = 2000))]
    pub description: Option<String>,
}
```

---

### VAL-002 - Sanitization Contenu HTML

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | CWE-79 (XSS) |
| Verification | Test injection XSS |
| Statut | A VERIFIER |

**Description :**
Le contenu utilisateur affiche DOIT etre sanitize pour prevenir les XSS.

---

### VAL-003 - Validation UUIDs

| Attribut | Valeur |
|----------|--------|
| Criticite | **SHOULD** |
| Risques adresses | CWE-20 |
| Verification | Test avec UUID invalide |
| Statut | IMPLEMENTE |

**Description :**
Les identifiants DEVRAIENT etre valides comme UUIDs avant traitement.

---

### VAL-004 - Audit Logging Structure

| Attribut | Valeur |
|----------|--------|
| Criticite | **SHOULD** |
| Risques adresses | RISK-SYS-001, THREAT-R-001 |
| Verification | Presence logs JSON |
| Statut | IMPLEMENTE |

**Description :**
Les actions sensibles DEVRAIENT etre loggees de maniere structuree.

**Implementation existante :**
```rust
// src/lib/audit.rs
AuditEvent::new("auth.login.success")
    .with_user(user_id)
    .with_metadata("ip", &ip)
    .log();
```

---

## Chiffrement

### CRYPTO-001 - Generation Cles RSA par Utilisateur

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | VULN-BASELINE-002, THREAT-S-003 |
| Verification | Verification table user_keys |
| Statut | IMPLEMENTE |

**Description :**
Chaque utilisateur DOIT avoir une paire de cles RSA-2048 generee a la creation.

**Implementation existante :**
```rust
// src/lib/crypto.rs
pub fn generate_rsa_keypair() -> AppResult<RsaKeyPair> {
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rand::thread_rng(), bits)?;
    // ...
}
```

---

### CRYPTO-002 - Cle Publique Authentique dans Actor

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | VULN-BASELINE-004, THREAT-S-004 |
| Verification | Fetch Actor et verification cle |
| Statut | IMPLEMENTE |

**Description :**
La cle publique exposee dans le profil Actor DOIT etre la vraie cle de l'utilisateur.

**Implementation existante :**
```rust
// src/api/activitypub.rs - get_actor
let public_key_pem = UserService::get_public_key(&state.db, id).await?;
```

---

### CRYPTO-003 - TLS 1.3 en Production

| Attribut | Valeur |
|----------|--------|
| Criticite | **MUST** |
| Risques adresses | CWE-319 |
| Verification | Test SSL Labs |
| Statut | DEPLOIEMENT |

**Description :**
Toutes les connexions en production DOIVENT utiliser TLS 1.3.

---

### CRYPTO-004 - Stockage Cle Privee Securise

| Attribut | Valeur |
|----------|--------|
| Criticite | **SHOULD** |
| Risques adresses | VULN-BASELINE-002 |
| Verification | Audit acces table user_keys |
| Statut | A AMELIORER |

**Description :**
Les cles privees DEVRAIENT etre chiffrees au repos en base de donnees.

---

## Conformite ASVS (Niveau L1)

| Categorie | Requis | Couvert | Statut |
|-----------|--------|---------|--------|
| V1 - Architecture | 6 | 4 | Partiel |
| V2 - Authentification | 7 | 3 | A implementer |
| V3 - Session | 4 | 2 | A implementer |
| V4 - Autorisation | 5 | 4 | OK |
| V5 - Validation | 5 | 4 | OK |
| V13 - API | 5 | 4 | OK |

---

## Prochaine Etape

Les exigences avec statut "A IMPLEMENTER" doivent etre ajoutees au backlog :

1. **Priorite P0** - Authentification complete (AUTH-002, AUTH-003)
2. **Priorite P1** - Roles et permissions (AUTHZ-003)
3. **Priorite P2** - Refresh tokens (AUTH-006)

```bash
# Generer les taches
/osk-tasks baseline

# Implementer
/osk-implement baseline
```
