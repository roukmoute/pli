---
id: kcyl5rz10
title: "Régler l'absence et la redirection"
depends_on: k7smvbih7
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/` avec client `server/src/infomaniak_api/` et fausse API de test, front SolidJS dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X09, ligne « Absence et transfert automatique ». Lis sur developer.infomaniak.com la documentation des réponses automatiques et des redirections d'une boîte avant de coder, ainsi que les pages d'aide citées par le cahier (répondeur, droits par boîte). Jalon Avancé.

## À faire
- Étendre le client Infomaniak : lire et écrire le message d'absence (texte, période, fuseau) et la redirection (adresse cible, conservation de la copie locale), selon ce que l'API propose réellement.
- Étendre la fausse API de test avec ces opérations et la simulation d'un droit manquant.
- Après chaque modification, relire le réglage effectif chez l'hébergeur et afficher celui-ci.
- Front : écran « Absence et redirection » dans les réglages du compte, les deux fonctions étant configurées séparément. Afficher l'adresse cible et le traitement de la copie locale.
- Droits contrôlés par fonction : si l'autorisation API manque, proposer le réglage chez le fournisseur quand l'utilisateur y a accès, sinon nommer le droit requis.
- Si une fonction n'est pas offerte par l'API, le dire dans l'écran et dans le journal de carte au lieu de la simuler.

## Critères d'acceptation
- [ ] Définir une absence avec période et fuseau, relire : le réglage affiché est celui de la fausse API.
- [ ] Configurer une redirection affiche sa cible et la conservation de copie.
- [ ] Un droit manquant affiche le réglage concerné et le droit requis.
- [ ] Les deux fonctions restent indépendantes.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Sur la boîte d'essai réelle : la réponse d'absence part pendant la période puis s'arrête après l'échéance ; une redirection atteint sa destination.

## Dépendances
« Gérer les règles de tri par l'API Infomaniak ».