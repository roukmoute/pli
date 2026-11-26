---
id: k1z2yvmib
title: Sauvegarder, restaurer et migrer le schéma
depends_on: kvogo9if0, khf35f65q, kt6ats33l, knehtpjgz, k7smvbih7
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/rusqlite dans `server/`, données dans `WEBMAIL_DATA_DIR` : base SQLite en WAL et fichiers). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte A05, ligne « Sauvegarde et reprise ». L'hébergeur reste la référence pour les messages ; ce qui n'existe que dans le webmail doit être sauvegardé. Jalon Parité : relis le schéma actuel avant de commencer.

## À faire
- Sous-commandes `webmail-server backup <dossier>` et `webmail-server restore <dossier>`. Sauvegarde cohérente à chaud (API de sauvegarde en ligne de SQLite) de la base et des fichiers, avec un manifeste (version de schéma, empreintes).
- Contenu garanti : préférences, décisions et destinations du Screener, piles, modèles s'ils existent, brouillons, file d'envoi et opérations en attente, fichiers, liens de partage avec leurs révocations, secrets chiffrés (la clé `WEBMAIL_SECRET_KEY` est sauvegardée à part, documenté).
- Restauration sur instance vierge : la projection des messages est reconstruite depuis IMAP ; les décisions et marques sont rapprochées ; les envois déjà acceptés ne repartent pas ; la copie locale des règles est relue chez Infomaniak avant d'être reconstruite.
- Migrations : test qui part d'une base figée d'une version antérieure (fichier d'essai dans `server/tests/fixtures/`) et la migre jusqu'au schéma courant sans perte.
- Refus clair de restaurer une sauvegarde d'un schéma plus récent que le binaire.
- Documentation d'exploitation dans `docs/exploitation/sauvegarde.md`.

## Critères d'acceptation
- [ ] Sur une instance vierge, brouillons, file d'envoi, décisions, piles, fichiers et droits de partage sont rétablis.
- [ ] Un expéditeur refusé le reste ; un lien révoqué reste inutilisable ; un fichier restauré se télécharge à l'identique.
- [ ] Un envoi accepté avant la sauvegarde garde son reçu et n'est pas renvoyé.
- [ ] La migration depuis la base figée passe.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Gérer les piles « Répondre plus tard » et « De côté » », « Reconstruire un dossier après changement d'UIDVALIDITY », « Traiter un envoi incertain ou partiellement refusé », « Partager un fichier par lien révocable », « Gérer les règles de tri par l'API Infomaniak ».