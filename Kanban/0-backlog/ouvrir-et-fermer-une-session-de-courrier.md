---
id: kaqobeby8
title: Ouvrir et fermer une session de courrier
depends_on: k0ptcluj1, k5q2ki3ia
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum dans `server/`, front SolidJS dans `web/`, banc IMAP/SMTP dans `bench/`, `Makefile`, `README.md`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F04, paragraphe « Connexion » et section « Confidentialité et qualité de service ». L'utilisateur se connecte avec son adresse complète et son secret de messagerie ; le profil serveur est choisi par le domaine de l'adresse.

## À faire
- Migrations : `users` (profil du webmail) et `accounts` (boîte rattachée, `user_id`, adresse, profil serveur). Les deux tables sont séparées dès maintenant pour le futur multicomptes ; une boîte par utilisateur pour l'instant.
- `POST /api/v1/session` : authentification par `LOGIN` IMAP avec `async-imap` 0.11.2 (feature `runtime-tokio`, vérifier la version). Succès : créer ou retrouver l'utilisateur et son compte, stocker le secret chiffré (AEAD, clé `WEBMAIL_SECRET_KEY`), poser un cookie de session `HttpOnly`, `SameSite=Strict`.
- `GET /api/v1/session`, `DELETE /api/v1/session` (la session est détruite côté serveur). Expiration configurable.
- Erreurs distinctes : `401 identifiants_invalides` et `503 service_indisponible`.
- Extracteur Axum `CompteAuthentifie` : toutes les routes de données l'exigent et filtrent par `account_id`. Documenter cette règle dans le README.
- Front : page de connexion (formulaire centré, marque, adresse, secret, réglage de langue accessible, Entrée soumet une seule fois, focus replacé après échec, adresse conservée, messages distincts). Actions `ouvrirSession` et `fermerSession` dans `web/src/sync/`. Une réponse 401 sur une session expirée ramène à la connexion.

## Critères d'acceptation
- [ ] Alice se connecte sur le banc et arrive sur la page provisoire de Réception.
- [ ] Mauvais secret : message d'identification ; banc arrêté : message de service indisponible.
- [ ] Une session expirée ou fermée revient à la connexion.
- [ ] Bob, connecté, n'obtient aucune donnée d'Alice (test d'API).
- [ ] Le secret n'apparaît ni en clair dans SQLite ni dans les journaux ; les journaux n'ont pas d'adresse complète.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Préparer le banc GreenMail et ses messages fictifs », « Configurer le profil serveur Infomaniak et son diagnostic ».