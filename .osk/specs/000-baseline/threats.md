# Modele de Menaces - Baseline

> Genere par `/osk-analyze baseline` le 2025-12-26
> Principes I (Modelisation menaces) et II (Analyse de risques)

## Resume Executif

| Categorie STRIDE | Nombre | Critique | Important | Moyen |
|------------------|--------|----------|-----------|-------|
| **S** - Spoofing | 4 | 2 | 1 | 1 |
| **T** - Tampering | 3 | 1 | 1 | 1 |
| **R** - Repudiation | 2 | 0 | 2 | 0 |
| **I** - Information Disclosure | 3 | 0 | 2 | 1 |
| **D** - Denial of Service | 2 | 0 | 1 | 1 |
| **E** - Elevation of Privilege | 2 | 0 | 1 | 1 |
| **Total** | **16** | **3** | **8** | **5** |

---

## Architecture et Trust Boundaries

```
                                 INTERNET
                                     |
                    +----------------+----------------+
                    |                |                |
                    v                v                v
            +-----------+    +-----------+    +-----------+
            |  Browser  |    | Fediverse |    | RSS Reader|
            |  (User)   |    | (Servers) |    |  (Anon)   |
            +-----+-----+    +-----+-----+    +-----+-----+
                  |                |                |
   ================== TRUST BOUNDARY #1 (TB1) ==================
                  |                |                |
                  v                v                v
            +---------------------------------------------+
            |              AXUM APPLICATION               |
            |  +---------+  +---------+  +---------+     |
            |  |REST API |  |ActivityPub|  |  Feeds  |     |
            |  | (JWT)   |  | (HTTP Sig)|  |(Public) |     |
            |  +----+----+  +----+----+  +----+----+     |
            |       |            |            |          |
            |       +------------+------------+          |
            |                    |                        |
            |            +-------v-------+               |
            |            |   Services    |               |
            |            +-------+-------+               |
            +--------------------+------------------------+
                                 |
        =============== TRUST BOUNDARY #2 (TB2) ===============
                                 |
            +--------------------+--------------------+
            |                    |                    |
            v                    v                    v
     +------------+       +------------+       +------------+
     | PostgreSQL |       |   MinIO    |       |  Fediverse |
     |  (Data)    |       |  (Files)   |       | (Federated)|
     +------------+       +------------+       +------------+
```

### Trust Boundaries

| ID | Nom | Description | Niveau Risque |
|----|-----|-------------|---------------|
| TB1 | Internet -> Application | Point d'entree public (REST, ActivityPub, Feeds) | CRITIQUE |
| TB2 | Application -> Data Stores | Acces base de donnees et stockage | IMPORTANT |
| TB3 | Application <-> Fediverse | Communication inter-instances | CRITIQUE |

---

## Menaces Detaillees par Categorie

### S - Spoofing (Usurpation d'identite)

#### THREAT-S-001 - JWT Secret Fallback

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 5/5 (Certaine si env manquant) |
| Impact | 5/5 (Compromission totale) |
| Asset Menace | Systeme d'authentification |
| Vulnerabilite | VULN-BASELINE-001 |

**Description :** Le code utilise un fallback `"dev-secret"` comme secret JWT si `JWT_SECRET` n'est pas defini. Un attaquant connaissant ce pattern peut forger des tokens d'authentification valides.

**Vecteur d'attaque :**
1. Detecter une instance sans JWT_SECRET configure
2. Generer un token JWT signe avec "dev-secret"
3. Acceder a n'importe quel compte utilisateur

**Localisation :** `src/api/middleware/auth.rs:87`

---

#### THREAT-S-002 - Signatures ActivityPub Non Verifiees

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 5/5 (Endpoints publics) |
| Impact | 4/5 (Pollution graphe social) |
| Asset Menace | Inbox ActivityPub |
| Vulnerabilite | VULN-BASELINE-003 |

**Description :** Les endpoints inbox acceptent les activites sans verifier la signature HTTP. Un attaquant peut envoyer des activites forgees au nom de n'importe quel acteur federe.

**Vecteur d'attaque :**
1. POST sur `/ap/users/:id/inbox` ou `/ap/inbox`
2. Envoyer une activite Follow/Like/Create avec actor falsifie
3. L'activite est traitee sans verification

**Localisation :** `src/api/activitypub.rs:80-81, 95-96`

---

#### THREAT-S-003 - Cle Privee Placeholder

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 4/5 (Visible dans code) |
| Impact | 4/5 (Forgery sortante) |
| Asset Menace | Signatures sortantes |
| Vulnerabilite | VULN-BASELINE-002 |

**Description :** La cle privee utilisee pour signer les requetes sortantes est un placeholder previsible. Les signatures sont invalides et potentiellement forgeables.

**Localisation :** `src/jobs/federation.rs:127`

---

#### THREAT-S-004 - Cle Publique Placeholder dans Actor

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 4/5 |
| Impact | 3/5 (Federation dysfonctionnelle) |
| Asset Menace | Profil Actor ActivityPub |
| Vulnerabilite | VULN-BASELINE-004 |

**Description :** La cle publique exposee dans le profil Actor est invalide. Les autres instances ne peuvent pas verifier les signatures.

**Localisation :** `src/api/activitypub.rs:61`

---

### T - Tampering (Alteration)

#### THREAT-T-001 - Injection Activites Forgees

| Attribut | Valeur |
|----------|--------|
| Severite | CRITIQUE |
| Probabilite | 5/5 |
| Impact | 4/5 |
| Asset Menace | Donnees sociales (follows, likes) |

**Description :** Sans verification de signature, des activites Create/Update/Delete peuvent etre injectees, alterant les donnees du graphe social.

**Vecteur d'attaque :**
1. Forger une activite Delete pour une recette distante
2. POST vers l'inbox sans signature
3. La recette est supprimee du cache local

---

#### THREAT-T-002 - Modification Recettes via IDOR

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 3/5 (Verification owner presente) |
| Impact | 4/5 |
| Asset Menace | Recettes utilisateur |

**Description :** Bien que des verifications owner_id existent, elles sont manuelles et pourraient etre oubliees dans de nouveaux endpoints.

**Localisation :** Pattern present dans `src/api/recipes.rs`

**Controles existants :** Verification `existing.author_id != auth.id`

---

#### THREAT-T-003 - Path Traversal Images

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 2/5 (UUID generes) |
| Impact | 3/5 |
| Asset Menace | Systeme de fichiers |

**Description :** Les cles de stockage images utilisent des UUID generes, mais la construction de cle pourrait etre vulnerable si modifiee.

**Controles existants :** `StorageClient::generate_image_key(recipe_id, "webp")` avec UUID

---

### R - Repudiation

#### THREAT-R-001 - Absence Audit Logging

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 5/5 (Aucun audit) |
| Impact | 3/5 (Non-conformite) |
| Asset Menace | Toutes les actions utilisateur |
| Vulnerabilite | RISK-SYS-001 |

**Description :** Le systeme utilise tracing pour le logging applicatif mais n'a pas de piste d'audit structuree pour les actions sensibles.

**Impact RGPD :** Non-conformite Article 30 (registre des activites de traitement)

---

#### THREAT-R-002 - Activites Federation Non Journalisees

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 5/5 |
| Impact | 3/5 |
| Asset Menace | Activites ActivityPub |

**Description :** Les activites recues et emises vers le Fediverse ne sont pas journalisees de maniere structuree, empechant l'investigation d'incidents.

---

### I - Information Disclosure

#### THREAT-I-001 - Recettes Privees via ActivityPub

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 4/5 |
| Impact | 3/5 |
| Asset Menace | Recettes avec visibility != public |

**Description :** Les endpoints `/ap/recipes/:id` et `/ap/books/:id` ne verifient pas la visibilite avant de retourner les objets, exposant potentiellement du contenu prive.

**Localisation :** `src/api/activitypub.rs`

---

#### THREAT-I-002 - Profils Publics Sans Restriction

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 5/5 |
| Impact | 2/5 |
| Asset Menace | Donnees profil utilisateur |

**Description :** Les profils utilisateur sont accessibles publiquement via ActivityPub. Cela peut exposer bio, display_name a des acteurs non authentifies.

**Note :** Comportement attendu pour ActivityPub, mais a documenter pour RGPD.

---

#### THREAT-I-003 - Erreurs Verboses

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 3/5 |
| Impact | 3/5 |
| Asset Menace | Details d'implementation |

**Description :** Les messages d'erreur pourraient reveler des details d'implementation (versions, chemins, requetes SQL) en production.

**Controle requis :** Sanitizer les erreurs en production

---

### D - Denial of Service

#### THREAT-D-001 - Rate Limiting Insuffisant

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 4/5 |
| Impact | 3/5 |
| Asset Menace | Disponibilite du service |

**Description :** Le rate limiting de 10 req/s peut etre insuffisant pour se proteger contre des attaques ciblees.

**Controle existant :** Rate limiting 10 req/s global

---

#### THREAT-D-002 - Inbox Flooding

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 4/5 |
| Impact | 3/5 |
| Asset Menace | Inbox ActivityPub |

**Description :** Les endpoints inbox n'ont pas de rate limiting specifique. Un acteur malveillant du Fediverse pourrait flooder l'inbox.

---

### E - Elevation of Privilege

#### THREAT-E-001 - IDOR Saved Recipes

| Attribut | Valeur |
|----------|--------|
| Severite | IMPORTANT |
| Probabilite | 3/5 |
| Impact | 3/5 |
| Asset Menace | Recettes sauvegardees d'autres utilisateurs |

**Description :** Les endpoints saved recipes doivent verifier l'ownership. La verification manuelle pourrait etre oubliee.

**Controle existant :** Verifications presentes mais a auditer

---

#### THREAT-E-002 - Absence Separation Admin/User

| Attribut | Valeur |
|----------|--------|
| Severite | MOYEN |
| Probabilite | 3/5 |
| Impact | 3/5 |
| Asset Menace | Fonctions administratives |

**Description :** Il n'y a pas de separation formelle entre roles admin et user. Un futur endpoint admin pourrait oublier la verification.

---

## Matrice de Risques

```
IMPACT
   ^
 5 |  M   H   [H]  C   C
 4 |  L   M   H   [H]  C
 3 |  L   L   M   H   H
 2 |  L   L   L   M   M
 1 |  L   L   L   L   M
   +----------------------------> PROBABILITE
       1   2   3   4   5

L = Low, M = Medium, H = High, C = Critical
[H] = Position actuelle du projet
```

---

## Prochaine Etape

-> Voir `risks.md` pour l'analyse detaillee et les plans de mitigation
-> Executer `/osk-harden` pour generer les taches de correction
