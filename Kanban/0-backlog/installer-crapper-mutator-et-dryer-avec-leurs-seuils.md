---
id: kmu2pmyh6
title: Installer crapper, mutator et dryer avec leurs seuils
depends_on: khfzz8cyo, kld8ie51k
---

## Contexte
Le propriétaire veut mesurer la qualité du code avec les trois outils de Robert C. Martin : `crapper` (score CRAP = complexité × manque de couverture), `mutator` (tests de mutation) et `dryer` (duplication structurelle). Ce sont des outils Python qui gèrent Rust et TypeScript. Ils n'ont **aucune version publiée** : il faut les figer sur un commit. Tous les agents devront ensuite lancer une commande unique avant de rendre leur carte.

Dépôts (licence 0BSD) : https://github.com/unclebob/crapper · https://github.com/unclebob/mutator · https://github.com/unclebob/dryer. **Lire leurs README actuels avant d'écrire quoi que ce soit** : les éléments ci-dessous datent du 2026-11-27 et sont à reconfirmer.

Ce qui a été relevé :
- Chaque outil se lance depuis la racine du projet ; le premier lancement crée un `.venv` et s'installe. Python 3 est requis.
- `mutator` exige que `crapper` soit cloné à côté de lui (`../crapper`).
- `crapper` : lit du LCOV (`cargo llvm-cov` pour Rust, `vitest --coverage` ou un script npm `coverage` pour TypeScript) ; `--threshold 30` sort en code 2 si le pire score dépasse le seuil ; `--changed`, `--use-existing-coverage` ; écrit `.metrics/crap.edn`.
- `mutator` : base `cargo test` et `npm test` ; accepte des chemins de fichiers, `--changed`, `--test-command`, `--max-workers` ; code 0 = tous les mutants tués, 2 = tests de base en échec, 3 = mutant survivant ; écrit `.metrics/mutate/`.
- `dryer` : `--threshold` (similarité, 0,82 par défaut), `--changed`, chemins en argument ; code 0 dès que le rapport est produit ; écrit `.metrics/dry.edn`.

## À faire
- Écrire `scripts/outils.sh` : clone `crapper`, `mutator` et `dryer` côte à côte dans `.outils/`, chacun sur le commit noté dans `scripts/outils.lock` (un outil et son SHA par ligne). Relancer le script ne change rien si tout est à jour. Le code des outils n'est pas copié dans le dépôt.
- Ajouter `.outils/`, `.metrics/` et les dossiers de couverture au `.gitignore`.
- Écrire `scripts/qualite.sh`, avec des sous-commandes :
  - `crap` : score CRAP sur Rust et TypeScript, seuil **30 bloquant** (code de sortie non nul au-delà).
  - `mutation [fichiers…]` : mutation sur les fichiers donnés ; sans argument, sur les fichiers source modifiés depuis le dernier tag git (`git describe --tags --abbrev=0`), puisqu'il y a un tag par carte. Code non nul si un mutant survit.
  - `mutation-complete` : mutation sur tout le code (lente, destinée à la CI nocturne).
  - `dry` : rapport de duplication, **jamais bloquant**.
  - sans sous-commande : `crap`, puis `mutation`, puis `dry`.
- Ajouter ce qu'il faut pour la couverture (`cargo-llvm-cov`, `@vitest/coverage-v8`, script npm `coverage`), en vérifiant les versions actuelles.
- Faire passer le code existant : ajouter les tests manquants ou simplifier une fonction si le seuil CRAP est dépassé ou si un mutant survit.
- Ajouter à `AGENTS.md` une section « Qualité avant de rendre une carte » :
  - lancer `scripts/qualite.sh` ;
  - CRAP > 30 : la carte ne peut pas être rendue ;
  - chaque mutant survivant est tué par un test, ou justifié nommément dans le journal de la carte (mutant équivalent, par exemple) ;
  - les candidats de `dryer` sont relus : factoriser, ou dire dans le journal pourquoi non.
  Faire `git pull --rebase` avant de pousser : une autre carte modifie `AGENTS.md` en parallèle.

## Critères d'acceptation
- [ ] Sur un clone neuf, `scripts/outils.sh` puis `scripts/qualite.sh` tournent sans intervention, avec les SHA de `scripts/outils.lock`.
- [ ] `scripts/qualite.sh crap` sort en code 0 sur le dépôt, et en code non nul si l'on ajoute une fonction complexe non testée (essai décrit dans le journal, non commité).
- [ ] `scripts/qualite.sh mutation` sort en code 0 sur le dépôt, et en code non nul si l'on retire une assertion qui tue un mutant (essai décrit dans le journal, non commité).
- [ ] `scripts/qualite.sh dry` sort toujours en code 0 et affiche son rapport.
- [ ] Rust et TypeScript sont tous deux analysés par les trois outils.
- [ ] `AGENTS.md` contient la section « Qualité avant de rendre une carte ».
- [ ] Les tests et le lint existants passent.

## Vérification
- `scripts/outils.sh && scripts/qualite.sh; echo $?`
- `scripts/qualite.sh crap; echo $?` · `scripts/qualite.sh mutation; echo $?` · `scripts/qualite.sh dry; echo $?`
- `git status --short` ne montre ni `.outils/` ni `.metrics/`.
- La commande unique de tests du projet.

## À vérifier par l'humain
- Python 3 est installé sur la machine.
- Le seuil CRAP de 30 et la durée de `scripts/qualite.sh` (notée dans le journal) conviennent.

## Dépendances
- « Écrire les règles de conception dans AGENTS.md ».
- « Mettre en place les tests » (commande unique, `cargo test`, Vitest).

Suivre les règles de commit, de push et de version décrites dans `AGENTS.md`.