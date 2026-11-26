---
id: kcd33glw2
title: "Mettre en place le pipeline d'acceptation APS"
depends_on: khfzz8cyo, kld8ie51k
---

## Contexte
Le propriétaire veut appliquer l'**Acceptance Pipeline Specification (APS)** de Robert C. Martin : les critères d'acceptation sont écrits en Gherkin, convertis en JSON, des tests d'acceptation en sont générés, puis une mutation des valeurs d'exemple vérifie que les scénarios sont réellement branchés sur l'application. Dans ce projet (webmail, back Rust/Axum, front SolidJS), les scénarios s'exécutent **au niveau de l'API HTTP du service Rust** ; l'interface reste testée par Playwright dans d'autres cartes.

Dépôt : https://github.com/unclebob/Acceptance-Pipeline-Specification. **Lire le README et les spécifications dans cet ordre** : `parser-spec.md`, `ir-dry-checker-spec.md`, `acceptance-generator.md`, `mutator-spec.md`. Le dépôt n'a pas de version publiée et sa licence n'a pas été trouvée : relever la licence dans le journal, ne pas copier son code dans le projet, le cloner sur un commit figé.

Ce qui a été relevé le 2026-11-27, à reconfirmer :
- Outils fournis, en Babashka (préféré) ou binaires Go (repli) : `bb gherkin-parser <feature> <json>`, `bb gherkin-ir-dry-checker [--include-exact] <json-ir> <rapport>`, `bb gherkin-mutator --runner-worker "<commande>" [options]`.
- À écrire dans le projet : le générateur de points d'entrée (`acceptance-entrypoint-generator <json-ir> <sortie>`), le runtime qui déroule les scénarios et distribue les étapes, les étapes du projet, et l'adaptateur de lancement qui exécute les tests générés contre un JSON fourni et répond succès, échec ou erreur d'infrastructure (il peut rester actif et recevoir les travaux de mutation sur stdin/stdout).
- Chemins usuels : `features/`, `build/acceptance/` (JSON et rapport DRY), `build/acceptance-mutation/`, `acceptance/generated/`.

## À faire
- Cloner le dépôt APS dans `.outils/aps` sur un commit figé, noté dans `scripts/outils.lock`. Si `scripts/outils.sh` existe déjà, l'étendre ; sinon le créer avec ce seul outil. Utiliser Babashka si `bb` est présent, sinon les binaires Go ; noter le choix dans le journal.
- Écrire le générateur, le runtime, l'adaptateur de lancement en Rust, dans l'espace de travail Cargo existant (adapter les chemins à la structure posée par le socle). Les tests générés lancent le service et l'appellent en HTTP ; le runtime lit le chemin du JSON à exécuter dans son environnement, pour que l'adaptateur puisse lui passer un JSON muté.
- Écrire `features/socle.feature` : un premier scénario de bout en bout sur ce que le socle expose déjà (par exemple la route de santé de l'API), avec au moins un `Scenario Outline` et sa table d'exemples, pour que la mutation ait des valeurs à muter. Écrire ses étapes.
- Écrire `scripts/acceptation.sh` : sans argument, parse toutes les `features/*.feature`, lance le contrôle DRY, génère les tests et les exécute ; `scripts/acceptation.sh mutation` lance la mutation des scénarios. Code de sortie non nul si un scénario échoue ou si un mutant survit.
- Ajouter `build/acceptance/`, `build/acceptance-mutation/` et `acceptance/generated/` au `.gitignore` (les tests générés sont reconstruits à chaque lancement).
- Ajouter à `AGENTS.md` une section « Tests d'acceptation (APS) » :
  - chaque carte de fonctionnalité traduit ses critères d'acceptation en scénarios Gherkin dans `features/`, un fichier par fonctionnalité, en français métier ;
  - les étapes passent par l'API HTTP, contre le banc IMAP/SMTP de test quand le scénario a besoin de courrier ;
  - lancer `scripts/acceptation.sh` avant de rendre la carte, et `scripts/acceptation.sh mutation` sur les scénarios ajoutés ;
  - comment ajouter une étape, avec un exemple.
  Faire `git pull --rebase` avant de pousser : une autre carte modifie `AGENTS.md` en parallèle.

## Critères d'acceptation
- [ ] `scripts/acceptation.sh` sort en code 0 sur un clone neuf et exécute le scénario de `features/socle.feature` contre le service réellement lancé.
- [ ] Modifier une valeur attendue dans `features/socle.feature` fait échouer `scripts/acceptation.sh` (essai décrit dans le journal, non commité).
- [ ] `scripts/acceptation.sh mutation` sort en code 0 : tous les mutants des valeurs d'exemple sont tués.
- [ ] Une étape Gherkin sans implémentation produit une erreur explicite qui cite son texte.
- [ ] Le générateur, le runtime et l'adaptateur ont leurs tests unitaires, et respectent les règles de conception de `AGENTS.md`.
- [ ] `AGENTS.md` contient la section « Tests d'acceptation (APS) ».
- [ ] Les tests et le lint existants passent.

## Vérification
- `scripts/acceptation.sh; echo $?`
- `scripts/acceptation.sh mutation; echo $?`
- La commande unique de tests du projet.
- `git status --short` ne montre ni `build/acceptance/` ni `acceptance/generated/`.

## À vérifier par l'humain
- Babashka (`bb`) est installé sur la machine, ou à défaut Go.
- La licence du dépôt APS relevée dans le journal convient pour cet usage.

## Dépendances
- « Écrire les règles de conception dans AGENTS.md ».
- « Mettre en place les tests ».

Suivre les règles de commit, de push et de version décrites dans `AGENTS.md`.