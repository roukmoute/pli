---
id: kgv7oozd2
title: Rattacher une deuxième boîte
depends_on: kaqobeby8, kmqbjnt1y, kd9c3y4qa
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/` avec tables `users` et `accounts` séparées, front SolidJS dans `web/`, cache IndexedDB cloisonné par compte). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte R04, ligne « Comptes liés et réception unifiée ». La fuite intercompte est une anomalie bloquante. Jalon Parité : relis le code actuel des sessions avant de commencer.

## À faire
- Un utilisateur peut posséder plusieurs comptes. La session désigne un utilisateur et un compte actif ; chaque requête de données nomme son compte et le serveur vérifie qu'il appartient à l'utilisateur.
- « Ajouter une boîte » depuis le menu de l'avatar : même authentification IMAP que la connexion ; le synchroniseur démarre pour la nouvelle boîte.
- Changement de compte depuis l'avatar : chaque compte garde sa vue, sa section et son point de lecture.
- Indépendance par compte : listes, flux d'événements et révisions, décisions du tri, brouillons, suggestions de destinataires, caches.
- Retirer une boîte : explique les effets locaux (caches, index, fichiers temporaires supprimés) et rappelle que la boîte continue de vivre chez son hébergeur.
- Tests avec Alice et Bob rattachés au même utilisateur, dont les dossiers portent les mêmes noms et les mêmes UID.

## Critères d'acceptation
- [ ] Deux boîtes aux mêmes noms de dossiers et mêmes UID gardent des listes, événements et brouillons indépendants.
- [ ] Un identifiant de message de la boîte A demandé sous le compte B est refusé.
- [ ] Changer de compte et revenir retrouve le même point de lecture.
- [ ] Retirer une boîte purge ses caches et son index, et ne touche pas au serveur IMAP.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Ouvrir et fermer une session de courrier », « Rattraper une interruption du flux d'événements », « Retrouver un brouillon après une coupure ».