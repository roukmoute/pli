---
id: ks1io8wuk
title: Mettre en place la CI GitHub Actions
depends_on: ki4xl91qt, kld8ie51k
---

## Contexte
Webmail Infomaniak à interface HEY : back Rust dans `server/`, front SolidJS/TypeScript dans `web/`, commandes dans le `Makefile` (`make lint`, `make build`, `make test`, `make check`). Le projet n'a pas de CI. Le dépôt sera hébergé sur GitHub.

## À faire
- Créer `.github/workflows/ci.yml`, déclenché sur `push` et `pull_request`.
- Job `check` sur `ubuntu-latest` : checkout, toolchain Rust lue depuis `rust-toolchain.toml` (avec `rustfmt` et `clippy`), Node 24, cache Cargo et npm, puis `make lint`, `make build`, `make test`.
- Vérifier sur internet les versions majeures actuelles des actions utilisées (`actions/checkout`, `actions/setup-node`, action de cache Rust) et les épingler.
- Aucune variable secrète : la CI n'accède jamais à une vraie boîte mail ni à `.env.recette`.
- Documenter dans le README que les cartes suivantes ajouteront leurs propres jobs (intégration avec Docker, bout en bout) dans ce même fichier.

## Critères d'acceptation
- [ ] Le workflow est syntaxiquement valide.
- [ ] Les commandes du job passent en local sur un clone propre.
- [ ] Aucun secret n'est requis.

## Vérification
```
docker run --rm -v "$PWD":/repo -w /repo rhysd/actionlint:latest
make lint && make build && make test
```

## À vérifier par l'humain
- Après le premier push sur GitHub, le workflow s'exécute et passe au vert.

## Dépendances
« Initialiser le dépôt git », « Mettre en place les tests ».