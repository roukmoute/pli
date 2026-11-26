---
id: k0j1wu20t
title: Ouvrir le webmail par connexion fédérée
depends_on: kaqobeby8, kgv7oozd2, kp6c7794x
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum dans `server/`, sessions avec `users` et `accounts`, administration dans `/admin`, front SolidJS dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X11, ligne « Services externes et connexion fédérée ». Le choix est **OpenID Connect générique**. Jalon Avancé : relis le code actuel des sessions avant de commencer.

## À faire
- Connexion OpenID Connect par flux « code d'autorisation » avec PKCE, via une bibliothèque Rust maintenue (vérifier laquelle et sa version). Configuration d'un fournisseur dans l'administration : émetteur, identifiant client, secret chiffré.
- Migration `federated_identities` : (émetteur, sujet) liés à un utilisateur du webmail.
- Distinguer trois accès : au webmail (identité fédérée), à la boîte IMAP/SMTP (secret de messagerie stocké chiffré), au service de fichiers. La connexion fédérée ouvre la session du webmail, elle ne remplace pas l'authentification IMAP.
- Une identité liée retrouve son profil et ses boîtes. Si l'autorisation de messagerie d'une boîte a expiré ou est refusée, indiquer quel compte reconnecter, sans bloquer les autres.
- Une identité inconnue suit la politique de création de profils de l'administration (création automatique ou refus).
- Bouton « Se connecter avec… » sur la page de connexion, affiché seulement si un fournisseur est configuré.
- Tests avec un fournisseur OpenID Connect local dans Docker (Dex ou équivalent léger, version vérifiée), ajouté à `bench/`.

## Critères d'acceptation
- [ ] Une identité liée ouvre la session et retrouve ses boîtes.
- [ ] Un secret de messagerie devenu invalide affiche le compte à reconnecter.
- [ ] Une identité inconnue est créée ou refusée selon la politique.
- [ ] `state`, `nonce` et PKCE sont vérifiés ; un retour falsifié est refusé.
- [ ] Sans fournisseur configuré, la page de connexion est inchangée.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Ouvrir et fermer une session de courrier », « Rattacher une deuxième boîte », « Administrer les comptes et diagnostiquer la synchronisation ».