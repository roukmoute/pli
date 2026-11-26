---
id: kpmdfz4yx
title: Tester le classement de bout en bout
depends_on: k7pfstynd, kmmb7jat5, kqgzqscrh, kut392580, kvogo9if0
---

## Contexte
Webmail Infomaniak à interface HEY (tests Playwright dans `e2e/`, lancés par `make e2e` sur le banc de `bench/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, section « Jalons et critères de livraison » (jalon Quotidien) et tableau « Navigation et classement ». Les fonctions de classement existent avec leurs tests unitaires ; cette carte les enchaîne dans un vrai navigateur et vérifie le résultat côté IMAP.

## À faire
- Utilitaire de test : second client IMAP qui lit l'état réel des dossiers du banc et peut y agir (simulateur du « second appareil »).
- Scénario sélection et archivage : ouvrir A, revenir, sélectionner B et C, archiver ; vérifier la liste, les compteurs et le dossier Archives réel.
- Scénario déplacement, copie, corbeille, restauration, vidage avec confirmation.
- Scénario dossiers et indésirables : créer et renommer un sous-dossier, y déplacer un message, « Déplacer vers Indésirables » puis en sortir.
- Scénario Flux et Justificatifs : destination ponctuelle d'un reçu sans effet sur les prochains messages de l'expéditeur.
- Scénario piles : deux fils dans « Répondre plus tard », un dans « De côté », retrait depuis un second onglet.
- Scénario second appareil : un déplacement fait par le client IMAP de test apparaît dans le webmail.
- Corriger les défauts mineurs trouvés ; lister les autres en fin de journal de carte.

## Critères d'acceptation
- [ ] Les six scénarios passent dans `make e2e`, sans écran.
- [ ] Chaque déplacement est vérifié dans le dossier IMAP réel, pas seulement à l'écran.
- [ ] Les scénarios sont indépendants (boîte repeuplée entre deux).
- [ ] Le job e2e de la CI les inclut.

## Vérification
```
make e2e
make check
```

## Dépendances
« Tester le jalon Lecture de bout en bout avec Playwright », « Supprimer, restaurer et vider la corbeille », « Organiser les dossiers et traiter les indésirables », « Lire le Flux et retrouver un justificatif », « Gérer les piles « Répondre plus tard » et « De côté » ».