---
id: ky9ino4na
title: Tester le jalon Avancé et rejouer les mesures de lecture
depends_on: krsvz68mw, k7qotjx6l, kl1g5x2nu, kjfitaoa4, kcyl5rz10, kpwtj6ty1, k0j1wu20t
---

## Contexte
Webmail Infomaniak à interface HEY (tests Playwright dans `e2e/`, `make e2e`, banc et fausses API dans `bench/`, mesures par `make perf` et rapports dans `docs/mesures/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, section « Jalons et critères de livraison » (jalon Avancé). Les fonctions avancées existent ; cette carte les enchaîne et vérifie qu'elles n'ont pas dégradé la première lecture.

## À faire
- Scénario : envoyer un message signé rédigé à partir d'un modèle ; le destinataire de test vérifie la signature.
- Scénario : programmer un envoi puis l'annuler ; aucun message n'arrive.
- Scénario : régler une absence et une redirection sur la fausse API Infomaniak et relire l'état.
- Scénario : panne du service de fichiers ; la lecture et la rédaction restent possibles.
- Scénario : connexion fédérée avec le fournisseur local ; le bon compte est retrouvé après reconnexion.
- Trace réseau : vérifier que les modules Fichiers et OpenPGP ne sont chargés qu'à leur première utilisation, et pas à l'ouverture de la Réception.
- Rejouer `make perf` avec toutes les fonctions activées et écrire un nouveau rapport daté qui compare les parcours de première lecture au rapport précédent.
- Corriger les défauts mineurs ; lister les autres en fin de journal de carte.

## Critères d'acceptation
- [ ] Les cinq scénarios passent dans `make e2e`, sans écran.
- [ ] Le test de trace réseau prouve le chargement à la demande de Fichiers et d'OpenPGP.
- [ ] Le nouveau rapport de mesures existe et signale toute régression de p95 par rapport au précédent.
- [ ] Le job e2e de la CI inclut les scénarios.

## Vérification
```
make e2e
make perf
make check
```

## À vérifier par l'humain
- Sur la boîte d'essai réelle : réponse d'absence et redirection effectives.

## Dépendances
« Mesurer les budgets de latence », « Rédiger à partir d'un modèle », « Programmer et annuler un envoi », « Signer, vérifier et chiffrer avec OpenPGP », « Régler l'absence et la redirection », « Joindre un fichier d'un service connecté », « Ouvrir le webmail par connexion fédérée ».