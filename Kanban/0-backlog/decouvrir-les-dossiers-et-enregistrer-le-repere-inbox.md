---
id: k2j1ykpie
title: Découvrir les dossiers et enregistrer le repère INBOX
depends_on: kaqobeby8
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum/Tokio/rusqlite dans `server/`, banc dans `bench/`, `Makefile`, `README.md`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F05, paragraphes « Premier rattachement » et « Dossiers et conversations », ligne « Tri des expéditeurs et vues du webmail » du tableau des données. Dès l'authentification, le synchroniseur doit connaître les dossiers et noter où commence le tri des nouveaux expéditeurs, avant tout import.

## À faire
- Créer le synchroniseur dans `server/src/sync/` : une tâche Tokio par compte, démarrée à l'ouverture de session et au démarrage du serveur pour les comptes connus, avec le secret déchiffré.
- Migration : `folders` (identifiant interne, compte, chemin distant, délimiteur, rôle, `uidvalidity`, `uidnext`, masqué) et `inbox_markers` (compte, `uidvalidity`, `uidnext` relevés au rattachement).
- Découverte par `LIST` : rôles Réception, Envoyés, Brouillons, Indésirables, Corbeille, Archives via SPECIAL-USE si annoncé, sinon par une table de libellés multilingues (Sent, Envoyés, Gesendet, Trash, Corbeille, Papierkorb…). Les dossiers personnels gardent un rôle vide.
- Enregistrer le repère d'INBOX (génération UIDVALIDITY et prochain UID) dans la même transaction que la découverte, avant tout import. L'opération est idempotente : un redémarrage ne déplace pas le repère.
- `GET /api/v1/folders` : dossiers du compte authentifié avec rôle et chemin.

## Critères d'acceptation
- [ ] Sur le jeu `base`, Envoyés et Corbeille sont reconnus malgré leurs libellés étrangers.
- [ ] Le dossier personnel est listé.
- [ ] Le repère d'INBOX existe avant qu'un seul message soit importé et ne change pas après redémarrage.
- [ ] Les dossiers d'un compte ne sont jamais renvoyés à un autre.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Ouvrir et fermer une session de courrier ».