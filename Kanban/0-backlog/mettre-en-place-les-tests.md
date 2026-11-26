---
id: kld8ie51k
title: Mettre en place les tests
depends_on: kdbrmneuv
---

## Contexte
Webmail Infomaniak à interface HEY : back Rust/Axum dans `server/`, front SolidJS/TypeScript dans `web/`, commandes dans le `Makefile` (voir `README.md`). Le socle existe mais n'a aucun test. Cette carte installe les frameworks et la commande unique que toutes les cartes suivantes utiliseront.

## À faire
- **Rust** : tests avec `cargo test`. Créer un utilitaire partagé (`server/tests/common/` ou module `test_support`) qui ouvre une base SQLite temporaire migrée et construit l'application Axum pour l'appeler en mémoire (`tower::ServiceExt::oneshot`).
- Premiers tests Rust : les migrations s'appliquent sur une base vide et sont rejouables ; `GET /api/v1/health` répond 200 ; le mode WAL est actif ; FTS5 est disponible (`CREATE VIRTUAL TABLE ... USING fts5`).
- **Front** : Vitest 5.0.x (vérifier la version courante) avec environnement DOM (`jsdom` ou `happy-dom`) et la bibliothèque de test Solid compatible avec la version de Solid installée. Premier test de composant et premier test du client `web/src/sync/api.ts` avec `fetch` simulé.
- **Makefile** : `make test` lance tous les tests unitaires (Rust + front) ; `make check` devient lint + build + test.
- Documenter dans le README comment écrire et lancer un test (un seul fichier, tout).

## Critères d'acceptation
- [ ] `make test` lance les tests Rust et front, et échoue si l'un échoue.
- [ ] Les quatre tests Rust et les deux tests front décrits existent et passent.
- [ ] `make check` inclut les tests.

## Vérification
```
make test
make check
```

## Dépendances
« Poser le socle Rust/Axum et SolidJS/Vite ».