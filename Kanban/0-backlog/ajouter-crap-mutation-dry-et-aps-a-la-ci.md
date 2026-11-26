---
id: kt4sr9hqb
title: Ajouter CRAP, mutation, DRY et APS à la CI
depends_on: kmu2pmyh6, kcd33glw2, ks1io8wuk
---

## Contexte
Le projet dispose de deux commandes de qualité : `scripts/qualite.sh` (score CRAP, tests de mutation, duplication, avec les outils `crapper`, `mutator`, `dryer`) et `scripts/acceptation.sh` (pipeline d'acceptation APS : scénarios Gherkin de `features/` exécutés contre l'API). Les outils sont clonés par `scripts/outils.sh` dans `.outils/`, sur les commits de `scripts/outils.lock`. La CI GitHub Actions existante lance déjà installation, lint, build et tests. Il faut y ajouter ces contrôles, sans rendre chaque push trop lent : les agents poussent directement sur `main`, plusieurs fois par carte.

## À faire
- Lire les workflows existants dans `.github/workflows/` et les scripts `scripts/outils.sh`, `scripts/qualite.sh`, `scripts/acceptation.sh` avant de modifier quoi que ce soit.
- À chaque push et pull request, ajouter des jobs :
  - **crap** : `scripts/qualite.sh crap`, bloquant (seuil 30).
  - **dry** : `scripts/qualite.sh dry`, non bloquant, rapport publié en artefact.
  - **acceptation** : `scripts/acceptation.sh`, bloquant.
- Créer un workflow nocturne (`schedule`, plus `workflow_dispatch` pour le lancer à la main) :
  - `scripts/qualite.sh mutation-complete` ;
  - `scripts/acceptation.sh mutation` ;
  - rapports publiés en artefacts ; le workflow échoue si un mutant survit.
- Installer dans les jobs ce que les scripts demandent : Python 3, Babashka (ou Go, selon le choix fait pour l'APS), `cargo-llvm-cov`. Vérifier les versions actuelles des actions utilisées et les figer.
- Mettre en cache `.outils/` avec une clé calculée sur `scripts/outils.lock`, ainsi que les caches Cargo et npm s'ils ne le sont pas déjà.
- Documenter dans `AGENTS.md` ce que la CI vérifie à chaque push et chaque nuit, en une courte liste.

## Critères d'acceptation
- [ ] Les fichiers de workflow sont valides (`actionlint` si disponible, sinon analyse YAML) et les jobs existants ne sont pas dégradés.
- [ ] Les jobs crap, dry et acceptation tournent à chaque push et pull request ; crap et acceptation sont bloquants, dry ne l'est pas.
- [ ] Le workflow nocturne existe, se lance aussi à la main, et couvre la mutation complète du code et la mutation des scénarios.
- [ ] Après le push, les jobs du push sont verts sur GitHub (`gh run list` / `gh run watch`) ; en cas d'échec, corriger avant de rendre la carte.
- [ ] Le workflow nocturne a été lancé une fois par `gh workflow run` et son résultat est noté dans le journal, avec sa durée.
- [ ] La CI n'utilise aucun secret de boîte réelle.

## Vérification
- `scripts/qualite.sh crap && scripts/qualite.sh dry && scripts/acceptation.sh` en local.
- `actionlint` (ou `python3 -c "import yaml,sys,glob; [yaml.safe_load(open(f)) for f in glob.glob('.github/workflows/*.yml')]"`).
- `gh run list --limit 5` puis `gh run watch <id>` après le push.
- `gh workflow run <workflow-nocturne>` puis `gh run watch <id>`.

## Dépendances
- « Installer crapper, mutator et dryer avec leurs seuils ».
- « Mettre en place le pipeline d'acceptation APS ».
- « Mettre en place la CI GitHub Actions ».

Suivre les règles de commit, de push et de version décrites dans `AGENTS.md`.