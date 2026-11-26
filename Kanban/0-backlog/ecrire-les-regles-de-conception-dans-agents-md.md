---
id: khfzz8cyo
title: Écrire les règles de conception dans AGENTS.md
depends_on: kufl8r92q, kdbrmneuv
---

## Contexte
Le projet est un webmail (back Rust/Axum, front SolidJS + TypeScript strict + Vite, lint Biome). Le propriétaire veut un code fondé sur la **composition d'objets**, avec le moins d'héritage possible. Les agents qui réalisent les autres cartes ne connaissent les règles du projet que par `AGENTS.md` (et `CLAUDE.md`, qui y renvoie) : c'est donc là qu'il faut les écrire, et les faire vérifier par l'outillage.

## À faire
- Ajouter à `AGENTS.md` une section « Règles de conception », sans toucher aux sections existantes (commits, versions) :
  - **TypeScript** : pas de `class … extends`. Seule exception : une classe d'erreur qui étend `Error`. Préférer fonctions, objets composés, injection des dépendances par paramètre, et unions discriminées plutôt que hiérarchies de classes.
  - **Rust** : petits traits ciblés, délégation à des champs plutôt que gros traits « fourre-tout », pas de simulation d'héritage par `Deref`, dépendances passées au constructeur.
  - Un exemple court « à éviter / à faire » pour chaque langage.
- Faire vérifier la règle TypeScript par le lint du front. Chercher d'abord dans la documentation actuelle de Biome s'il existe une règle intégrée ou un plugin GritQL adapté ; sinon écrire un petit script de vérification (par exemple `scripts/verifier-extends.sh`) appelé par la commande de lint existante. L'exception `extends Error` doit passer.
- Corriger le code existant s'il enfreint la règle.
- Vérifier que `CLAUDE.md` renvoie bien à `AGENTS.md` ; ne pas dupliquer les règles.

## Critères d'acceptation
- [ ] `AGENTS.md` contient la section « Règles de conception » avec les règles TypeScript et Rust et leurs exemples.
- [ ] Un fichier TypeScript contenant `class A extends B {}` fait échouer la commande de lint du front ; `class MonErreur extends Error {}` ne la fait pas échouer. Un test automatisé ou un fichier témoin le prouve.
- [ ] La commande de lint et les tests existants passent sur le dépôt.
- [ ] Le journal de la carte indique le mécanisme retenu (règle Biome, plugin GritQL ou script) et pourquoi.

## Vérification
- La commande de lint du front documentée dans `AGENTS.md` ou le `README` (ex. `npm run lint`).
- La commande unique de tests du projet.
- `grep -n "Règles de conception" AGENTS.md`

## Dépendances
- « Relier le dépôt à GitHub et fixer commits et versions » (crée `AGENTS.md`).
- « Poser le socle Rust/Axum et SolidJS/Vite » (pose Biome et la commande de lint).

Suivre les règles de commit, de push et de version décrites dans `AGENTS.md`.