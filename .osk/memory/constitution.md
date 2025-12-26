# Constitution Sécurité

> Générée par `/osk-configure` le 2025-12-26
> Adaptation des 7 principes OpenSecKit à ce projet

## Principes Fondamentaux

Ce projet applique les **7 principes constitutionnels OpenSecKit**, pondérés selon le contexte validé.

## Pondération des Principes

> Validée par l'utilisateur le 2025-12-26

| # | Principe | Priorité | Justification |
|---|----------|----------|---------------|
| I | Threat Modeling | **Critical** | API publique REST + ActivityPub fédéré exposé à Internet |
| II | Risk Analysis | High | RGPD applicable, données utilisateur à protéger |
| III | Security Requirements | High | Données personnelles, authentification JWT, fédération |
| IV | Security Testing | High | CI/CD existant avec cargo-audit, tests intégration |
| V | Secrets Management | **Critical** | Secrets en .env, fallbacks dangereux détectés dans le code |
| VI | Audit Logging | High | RGPD requiert traçabilité des accès aux données personnelles |
| VII | Patch Management | Medium | Cargo.lock récent, audit automatisé en CI |

## Exigences par Domaine

### RGPD - Exigences Activées

- [ ] **Art. 5** : Principes de traitement - Minimisation des données, limitation de conservation
- [ ] **Art. 6** : Base légale - Documenter la base légale pour chaque traitement
- [ ] **Art. 12-14** : Transparence - Informer les utilisateurs sur le traitement de leurs données
- [ ] **Art. 15-22** : Droits des personnes - Implémenter accès, rectification, effacement, portabilité
- [ ] **Art. 25** : Privacy by Design - Intégrer la protection des données dès la conception
- [ ] **Art. 30** : Registre des traitements - Documenter les activités de traitement
- [ ] **Art. 32** : Sécurité du traitement - Mesures techniques et organisationnelles appropriées
- [ ] **Art. 33-34** : Notification de violation - Procédure en cas de fuite de données

## Règles Projet

### Règles Critiques (non négociables)

- Jamais de secret en dur dans le code source
- Jamais de fallback secret en production (JWT_SECRET, clés privées)
- Toutes les entrées utilisateur doivent être validées
- Les requêtes SQL doivent utiliser des paramètres (SQLx query_as!)
- Les données personnelles doivent être supprimables (droit à l'effacement)

### Règles Standard

- Utiliser tracing pour la journalisation
- Valider les modèles avec la crate validator
- Échapper le HTML/XML en sortie
- Documenter les flux de données personnelles
- Revoir les dépendances pour les CVE (cargo-audit)
