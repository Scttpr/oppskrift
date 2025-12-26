# Vulnérabilités Baseline

> Généré par `/osk-baseline` le 2025-12-26
> Vulnérabilités détectées lors de l'analyse initiale

## Résumé

| Sévérité | Nombre |
|----------|--------|
| CRITIQUE | 3 |
| IMPORTANT | 1 |
| MOYEN | 0 |
| MINEUR | 0 |
| **Total** | **4** |

---

## VULN-BASELINE-001 - JWT Secret Fallback

### Identification

| Attribut | Valeur |
|----------|--------|
| ID | VULN-BASELINE-001 |
| Sévérité | CRITIQUE |
| Priorité | P0 |
| Échéance SLA | 48h |
| Statut | OUVERT |

### Localisation

- **Fichier** : `src/api/middleware/auth.rs`
- **Ligne** : 87
- **Code** :
```rust
let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret".to_string());
```

### Description

Le code utilise un fallback `"dev-secret"` si la variable d'environnement `JWT_SECRET` n'est pas définie. En production, si la variable est oubliée, tous les tokens JWT seront signés avec un secret prévisible, permettant la forgery de tokens d'authentification.

### Impact

- **Confidentialité** : Totale (accès à tous les comptes)
- **Intégrité** : Totale (modification de toutes les données)
- **Disponibilité** : Partielle (actions destructives possibles)

### Classification

| Standard | Référence |
|----------|-----------|
| CWE | CWE-798 (Use of Hard-coded Credentials) |
| OWASP | A07:2021 (Identification and Authentication Failures) |
| Principe violé | V (Secrets Management) |

### Contrôles Requis

1. Remplacer `unwrap_or_else` par `expect("JWT_SECRET must be set")`
2. Ajouter validation au démarrage de l'application
3. Documenter dans `.env.example` et DEPLOYMENT.md

---

## VULN-BASELINE-002 - Clé Privée Placeholder ActivityPub

### Identification

| Attribut | Valeur |
|----------|--------|
| ID | VULN-BASELINE-002 |
| Sévérité | CRITIQUE |
| Priorité | P0 |
| Échéance SLA | 48h |
| Statut | OUVERT |

### Localisation

- **Fichier** : `src/jobs/federation.rs`
- **Ligne** : 127
- **Code** :
```rust
"placeholder_private_key".to_string(), // TODO: Get from DB
```

### Description

La clé privée utilisée pour signer les requêtes HTTP sortantes vers le Fediverse est un placeholder. Cela rend toutes les signatures invalides et empêche la fédération de fonctionner. Plus grave, si un attaquant connaît ce placeholder, il pourrait forger des requêtes.

### Impact

- **Confidentialité** : Moyenne (signatures prévisibles)
- **Intégrité** : Élevée (forgery de requêtes possibles)
- **Disponibilité** : Totale sur fédération (non fonctionnelle)

### Classification

| Standard | Référence |
|----------|-----------|
| CWE | CWE-321 (Use of Hard-coded Cryptographic Key) |
| OWASP | A02:2021 (Cryptographic Failures) |
| Principe violé | V (Secrets Management) |

### Contrôles Requis

1. Générer une paire RSA par utilisateur à la création
2. Stocker la clé privée chiffrée en base de données
3. Récupérer la vraie clé privée lors de la signature

---

## VULN-BASELINE-003 - Signature HTTP Non Vérifiée

### Identification

| Attribut | Valeur |
|----------|--------|
| ID | VULN-BASELINE-003 |
| Sévérité | CRITIQUE |
| Priorité | P0 |
| Échéance SLA | 48h |
| Statut | OUVERT |

### Localisation

- **Fichier** : `src/api/activitypub.rs`
- **Lignes** : 80-81, 95-96
- **Code** :
```rust
// TODO: Verify HTTP signature from headers
// let signature = headers.get("signature");
```

### Description

Les endpoints inbox (individuel et partagé) acceptent les activités ActivityPub sans vérifier la signature HTTP. Un attaquant peut envoyer des activités forgées (Follow, Like, Create) au nom de n'importe quel acteur fédéré.

### Impact

- **Confidentialité** : Faible
- **Intégrité** : Totale (pollution du graphe social, spam)
- **Disponibilité** : Moyenne (DoS par injection d'activités)

### Classification

| Standard | Référence |
|----------|-----------|
| CWE | CWE-347 (Improper Verification of Cryptographic Signature) |
| OWASP | A02:2021 (Cryptographic Failures) |
| Principe violé | III (Security Requirements) |

### Contrôles Requis

1. Implémenter parsing du header Signature
2. Récupérer la clé publique de l'acteur (fetch Actor)
3. Vérifier la signature avant traitement
4. Rejeter avec 401 si signature invalide

---

## VULN-BASELINE-004 - Clé Publique Placeholder

### Identification

| Attribut | Valeur |
|----------|--------|
| ID | VULN-BASELINE-004 |
| Sévérité | IMPORTANT |
| Priorité | P1 |
| Échéance SLA | 7 jours |
| Statut | OUVERT |

### Localisation

- **Fichier** : `src/api/activitypub.rs`
- **Ligne** : 61
- **Code** :
```rust
let public_key_pem = "-----BEGIN PUBLIC KEY-----\nPLACEHOLDER\n-----END PUBLIC KEY-----";
```

### Description

La clé publique exposée dans le profil Actor ActivityPub est un placeholder invalide. Les autres instances ne peuvent pas vérifier les signatures des messages sortants, et pourraient même parser cette clé invalide causant des erreurs.

### Impact

- **Confidentialité** : Faible
- **Intégrité** : Moyenne (fédération dysfonctionnelle)
- **Disponibilité** : Élevée sur fédération

### Classification

| Standard | Référence |
|----------|-----------|
| CWE | CWE-321 (Use of Hard-coded Cryptographic Key) |
| OWASP | A02:2021 (Cryptographic Failures) |
| Principe violé | V (Secrets Management) |

### Contrôles Requis

1. Générer une vraie paire RSA à la création de l'utilisateur
2. Stocker la clé publique en base de données
3. Retourner la vraie clé publique dans le profil Actor

---

## Prochaine Étape

Exécuter `/osk-harden` pour générer les tâches de correction de ces vulnérabilités.
