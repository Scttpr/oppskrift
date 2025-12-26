# Analyse RGPD - Baseline

> Genere par `/osk-analyze baseline` le 2025-12-26
> Extension domaine RGPD (Standard)

## Resume Executif

| Conformite | Score |
|------------|-------|
| Articles fondamentaux | 40% |
| Droits des personnes | 25% |
| Securite des traitements | 35% |
| **Global RGPD** | **33%** |

---

## Donnees Personnelles Identifiees

### Inventaire

| Categorie | Donnees | Table | Sensibilite | Base Legale |
|-----------|---------|-------|-------------|-------------|
| Identite | username | users | Standard | Contrat |
| Identite | display_name | users | Standard | Contrat |
| Contact | email (si present) | users | Standard | Contrat |
| Profil | bio | users | Standard | Consentement |
| Profil | avatar_url | users | Standard | Consentement |
| Contenu | recettes | recipes | Standard | Contrat |
| Social | follows | follows | Standard | Consentement |
| Social | likes | recipe_likes | Standard | Consentement |
| Social | saved_recipes | saved_recipes | Standard | Consentement |
| Technique | created_at, updated_at | * | Standard | Interet legitime |
| Federation | public_key | users (a creer) | Standard | Interet legitime |

### Flux de Donnees

```
+------------+     +----------------+     +------------+
|  Personne  | --> | Oppskrift      | --> | Fediverse  |
| concernee  |     | (responsable)  |     | (tiers)    |
+------------+     +----------------+     +------------+
      ^                   |                     |
      |                   v                     v
      |            +------------+        +-----------+
      +------------|    DB      |        | Instances |
       (droits)    | PostgreSQL |        | distantes |
                   +------------+        +-----------+
```

---

## Articles Applicables

### Article 5 - Principes

| Principe | Statut | Observation |
|----------|--------|-------------|
| Liceite | OK | Bases legales identifiees |
| Finalite | OK | Usage clair (partage recettes) |
| Minimisation | A VERIFIER | Champs bio/display_name optionnels |
| Exactitude | A IMPLEMENTER | Pas de mecanisme de mise a jour |
| Conservation | A IMPLEMENTER | Pas de politique de retention |
| Integrite | A RISQUE | Secrets non securises |
| Confidentialite | A RISQUE | Audit logging absent |

### Article 6 - Bases Legales

| Traitement | Base Legale | Justification |
|------------|-------------|---------------|
| Creation compte | Contrat | Necessaire au service |
| Stockage recettes | Contrat | Fonctionnalite principale |
| Relations sociales | Consentement | Action explicite (follow) |
| Federation | Interet legitime | Nature du service |
| Logs applicatifs | Interet legitime | Securite du service |

### Articles 12-22 - Droits des Personnes

| Droit | Article | Statut | Implementation |
|-------|---------|--------|----------------|
| Information | 13-14 | A FAIRE | Politique de confidentialite |
| Acces | 15 | PARTIEL | API GET user, manque export |
| Rectification | 16 | OK | API PATCH user |
| Effacement | 17 | A VERIFIER | Cascade sur FK, federation? |
| Portabilite | 20 | A FAIRE | Export JSON/ActivityPub |
| Opposition | 21 | A FAIRE | Opt-out federation |

---

## Risques RGPD Identifies

### RGPD-RISK-001 - Absence Export Donnees

| Attribut | Valeur |
|----------|--------|
| Article | 20 (Portabilite) |
| Severite | IMPORTANT |
| Impact | Amende jusqu'a 20M EUR |
| Probabilite | Moyenne |

**Description :** Aucun mecanisme d'export des donnees personnelles au format structure.

**Controle requis :**
- Endpoint `GET /api/v1/users/me/export`
- Format JSON + ActivityPub compatible

---

### RGPD-RISK-002 - Federation Sans Consentement Explicite

| Attribut | Valeur |
|----------|--------|
| Article | 6 (Base legale) |
| Severite | MOYEN |
| Impact | Contestation base legale |
| Probabilite | Faible |

**Description :** Les donnees sont federees vers d'autres instances sans consentement explicite au-dela de l'acceptation des CGU.

**Controle requis :**
- Mention claire dans CGU
- Option opt-out de federation

---

### RGPD-RISK-003 - Absence Politique Retention

| Attribut | Valeur |
|----------|--------|
| Article | 5(1)(e) (Conservation limitee) |
| Severite | IMPORTANT |
| Impact | Non-conformite |
| Probabilite | Moyenne |

**Description :** Pas de politique de retention des donnees definie. Les donnees sont conservees indefiniment.

**Controle requis :**
- Definir durees de retention
- Implementer purge automatique

---

### RGPD-RISK-004 - Effacement Incomplet (Droit a l'Oubli)

| Attribut | Valeur |
|----------|--------|
| Article | 17 (Effacement) |
| Severite | IMPORTANT |
| Impact | Violation droits |
| Probabilite | Moyenne |

**Description :** La suppression d'un utilisateur ne garantit pas l'effacement des donnees federees vers d'autres instances.

**Controle requis :**
- Emettre activite Delete vers followers
- Documenter limitation (federation)

---

### RGPD-RISK-005 - Absence Audit Logging

| Attribut | Valeur |
|----------|--------|
| Article | 30 (Registre des activites) |
| Severite | IMPORTANT |
| Impact | Non-conformite |
| Probabilite | Certaine |

**Description :** Aucune trace d'audit des acces et modifications des donnees personnelles.

**Controle requis :**
- Audit logging structure
- Retention 3 ans

---

## Actions Requises

### P1 - Court Terme (7j)

| ID | Action | Article | Effort |
|----|--------|---------|--------|
| RGPD-001 | Rediger politique de confidentialite | 13-14 | 4h |
| RGPD-002 | Documenter bases legales | 6 | 2h |
| RGPD-003 | Implementer audit logging | 30 | Voir FIX-004 |

### P2 - Moyen Terme (30j)

| ID | Action | Article | Effort |
|----|--------|---------|--------|
| RGPD-004 | Endpoint export donnees | 20 | 4h |
| RGPD-005 | Definir politique retention | 5(1)(e) | 2h |
| RGPD-006 | Documenter flux federation | 13 | 2h |

### P3 - Long Terme (90j)

| ID | Action | Article | Effort |
|----|--------|---------|--------|
| RGPD-007 | Opt-out federation | 21 | 8h |
| RGPD-008 | Purge automatique | 5(1)(e) | 4h |
| RGPD-009 | Delete federe | 17 | 4h |

---

## Registre des Traitements (Article 30)

| Traitement | Finalite | Categories | Destinataires | Transferts | Retention |
|------------|----------|------------|---------------|------------|-----------|
| Gestion comptes | Service | Identite, Profil | Oppskrift | Non | Compte actif |
| Recettes | Service | Contenu | Oppskrift, Fediverse | Oui (AP) | Indefini |
| Relations sociales | Service | Social | Oppskrift, Fediverse | Oui (AP) | Relation active |
| Logs | Securite | Technique | Oppskrift | Non | A definir |

---

## Documentation Requise

1. **Politique de confidentialite** - A creer dans `/docs/legal/`
2. **Mentions legales** - A creer
3. **Registre des traitements** - Ce document
4. **DPIA** (si necessaire) - A evaluer apres croissance

---

## Prochaine Etape

-> Integrer actions RGPD dans le plan de correction global
-> Creer `/docs/legal/privacy-policy.md`
