---
id: kufl8r92q
title: Relier le dépôt à GitHub et fixer commits et versions
depends_on: ki4xl91qt
---

## Contexte
Le projet (un webmail Rust/Axum + SolidJS, cahier des charges dans `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`) vient d'être initialisé en dépôt git local par la carte « Initialiser le dépôt git ». Le propriétaire veut que tout le travail soit publié sur le dépôt GitHub public https://github.com/roukmoute/pli, directement sur `main`, en commits petits et atomiques, avec une nouvelle version publiée à chaque carte terminée.

Cette carte raccorde le dépôt local à GitHub et écrit ces règles dans le dépôt. Les agents qui réaliseront les autres cartes ne connaissent pas cette décision : ils ne la découvriront que par `AGENTS.md` et `CLAUDE.md`.

État connu du dépôt distant (à revérifier) : il contient un seul commit sur `main`, avec un fichier `LICENSE` (MIT). Aucun tag, aucune release.

## À faire
1. **Vérifier les accès.** `git ls-remote` sur le dépôt doit répondre (essayer `git@github.com:roukmoute/pli.git`, sinon `https://github.com/roukmoute/pli.git`) et `gh auth status` doit indiquer un compte connecté avec droit d'écriture. Si l'un des deux échoue, ne rien contourner : s'arrêter et l'expliquer dans le journal de la carte.
2. **Vérifier ce qui va être publié.** Le dépôt est public. Avant le premier push, contrôler avec `git ls-files` qu'aucun fichier d'environnement ou secret n'est suivi (`.env`, `.env.*`, `.env.recette`, clés) et que `.gitignore` les exclut, ainsi que `Kanban/.runs/`, `Kanban/.leases/`, `Kanban/.downloads/` et `Kanban/.lead/`. Corriger `.gitignore` si besoin.
3. **Raccorder.** Ajouter le remote `origin` avec l'URL qui a répondu à l'étape 1, puis `git fetch origin`. Rejouer les commits locaux au-dessus de `origin/main` (`git rebase origin/main`) pour conserver le commit `LICENSE` existant. Si un fichier `LICENSE` local entre en conflit, garder celui du dépôt distant. Pousser avec `git push -u origin main`. **Aucun push forcé**, à aucun moment.
4. **Écrire `AGENTS.md`** à la racine, en français, avec une section « Commits, push et versions » qui contient les règles ci-dessous. Si le fichier existe déjà, ajouter la section sans toucher au reste.
5. **Écrire `CLAUDE.md`** à la racine : une ligne qui demande de lire et d'appliquer `AGENTS.md` (ou l'import `@AGENTS.md`). S'il existe déjà, y ajouter ce renvoi.
6. **Écrire `scripts/version.sh`** (bash, exécutable), décrit plus bas, puis publier la première version avec ce script : `v0.1.0`.

### Règles à écrire dans `AGENTS.md`
- **Commits petits et atomiques** : un commit = un seul changement cohérent, qui laisse le projet dans un état qui compile et dont les tests passent. Ne pas mélanger refactorisation, fonctionnalité et mise en forme dans un même commit. Une carte produit en général plusieurs commits.
- **Format Conventional Commits** : `type(portée): résumé` à l'impératif, 72 caractères au plus pour la première ligne. Types admis : `feat`, `fix`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`, `perf`. Donner deux ou trois exemples.
- **Push direct sur `main`** : pas de branche de travail, pas de pull request. Avant chaque push : `git pull --rebase origin main`, puis `git push origin main`. Si le push est refusé parce qu'un autre agent a poussé entre-temps, refaire `git pull --rebase` puis réessayer. Jamais de `--force` ni de `--force-with-lease`.
- **Deux agents peuvent travailler en même temps** sur ce dépôt : ne committer que les fichiers de sa propre carte (`git add` ciblé, jamais `git add -A` à l'aveugle), et ne jamais réécrire un commit déjà poussé.
- **Une version par carte terminée** : quand tous les critères de la carte sont remplis et que tout est poussé, lancer `scripts/version.sh` une seule fois, juste avant de remettre la carte en validation. Il crée le tag suivant et la Release GitHub publique avec notes générées. Noter le numéro de version dans le journal de la carte.
- **Dépôt public** : ne jamais committer de secret, de fichier `.env*`, ni d'adresse ou de contenu de courrier réel.

### Comportement de `scripts/version.sh`
- Refuse de tourner si l'arbre de travail a des modifications non commitées, si la branche courante n'est pas `main`, ou si `HEAD` n'est pas identique à `origin/main` après `git fetch origin --tags` (message d'erreur clair, code de sortie non nul).
- Si `HEAD` porte déjà un tag `vX.Y.Z`, ne crée rien, affiche ce tag et sort avec le code 0 (relance sans effet).
- Calcule la version suivante à partir du plus grand tag `vX.Y.Z` connu du dépôt distant : incrémente le numéro mineur et remet le correctif à 0 (`v0.1.0` → `v0.2.0`). Sans aucun tag : `v0.1.0`.
- Crée un tag annoté, le pousse avec `git push origin <tag>`, puis crée la Release avec `gh release create <tag> --generate-notes --verify-tag`.
- Si le push du tag est refusé parce qu'un autre agent a pris ce numéro, supprime le tag local, refait `git fetch --tags`, recalcule et réessaie (5 tentatives au plus).
- Option `--dry-run` : affiche la version qui serait créée, sans rien créer ni pousser.
- Affiche la version créée sur la sortie standard.

## Critères d'acceptation
- [ ] `origin` pointe sur `roukmoute/pli` et `main` suit `origin/main`.
- [ ] Le commit `LICENSE` d'origine du dépôt distant est toujours dans l'historique de `main`, et les commits locaux sont au-dessus. Aucun push forcé n'a été fait.
- [ ] Aucun fichier d'environnement ou secret n'est suivi par git.
- [ ] `AGENTS.md` contient toutes les règles listées ci-dessus, et `CLAUDE.md` y renvoie.
- [ ] `scripts/version.sh` est exécutable, passe `bash -n` (et `shellcheck` s'il est installé), et respecte chaque point de son comportement décrit.
- [ ] Le tag `v0.1.0` et sa Release publique avec notes générées existent sur GitHub et pointent sur le dernier commit de `main`.
- [ ] Les commits de cette carte respectent eux-mêmes les règles (atomiques, Conventional Commits).

## Vérification
```sh
git remote get-url origin
git fetch origin --tags && git status -sb          # main...origin/main, rien en avance ni en retard
git log --oneline origin/main | tail -n 3          # le commit LICENSE d'origine est présent
git ls-files | grep -Ei '(^|/)\.env' ; echo "code=$?"   # attendu : aucune ligne, code=1
bash -n scripts/version.sh
scripts/version.sh --dry-run                       # relancé après la publication : affiche v0.1.0 déjà posé sur HEAD
git ls-remote --tags origin                        # contient refs/tags/v0.1.0
gh release view v0.1.0 --repo roukmoute/pli
```

## À vérifier par l'humain
- La page https://github.com/roukmoute/pli/releases affiche la Release `v0.1.0` avec ses notes.
- Le contenu publié (dont le dossier `Kanban/`) est bien celui qu'il accepte de rendre public.

## Dépendances
- « Initialiser le dépôt git » (le dépôt local et son premier commit doivent exister).

Cette carte doit être terminée avant « Poser le socle Rust/Axum et SolidJS/Vite » et toutes les suivantes, pour que leurs agents trouvent les règles dans `AGENTS.md`.