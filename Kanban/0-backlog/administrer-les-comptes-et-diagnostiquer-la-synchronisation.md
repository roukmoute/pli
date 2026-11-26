---
id: kp6c7794x
title: Administrer les comptes et diagnostiquer la synchronisation
depends_on: k5q2ki3ia, kmqbjnt1y, kt6ats33l, kqhbe3h9k
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, API `/api/admin/v1` protégée par `WEBMAIL_ADMIN_TOKEN`, page `/admin` dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte A04, section « Administration et exploitation ». Jalon Parité : relis le code actuel de l'administration et du synchroniseur avant de commencer.

## À faire
- Utilisateurs et profils : associer domaines et profils serveur, décider de l'auto-provisionnement, autoriser ou non les boîtes externes, afficher le nombre de profils actifs.
- Avant de désactiver un profil : montrer les comptes concernés et demander confirmation.
- Exploitation, par compte : dernière synchronisation réussie, nombre d'actions en attente, retard d'indexation, erreurs SMTP classées, espace utilisé (base, corps, fichiers), taille du WAL.
- Actions : resynchronisation ciblée d'un dossier, reconstruction de l'index de recherche d'un compte. Les données déjà disponibles restent consultables pendant l'opération.
- Journaux consultables depuis l'administration.
- Règle de confidentialité : métriques et journaux sans adresse complète, objet, corps, jeton ni mot de passe. Les comptes y sont désignés par un identifiant interne. L'administration ne donne accès au contenu d'aucune boîte.
- Point de métriques `GET /api/admin/v1/metrics` au format texte.

## Critères d'acceptation
- [ ] Un retard de synchronisation provoqué en test est visible ; relancer le dossier ramène l'état à jour.
- [ ] Reconstruire un index n'empêche pas de lire la Réception.
- [ ] Désactiver un profil liste d'abord les comptes touchés.
- [ ] Un test parcourt journaux et métriques et n'y trouve ni adresse complète ni objet des messages du banc.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Configurer le profil serveur Infomaniak et son diagnostic », « Rattraper une interruption du flux d'événements », « Traiter un envoi incertain ou partiellement refusé », « Chercher dans le dossier actif avec FTS5 ».