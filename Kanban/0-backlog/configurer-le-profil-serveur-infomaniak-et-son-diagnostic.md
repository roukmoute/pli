---
id: k5q2ki3ia
title: Configurer le profil serveur Infomaniak et son diagnostic
depends_on: kld8ie51k, kmu2pmyh6, kcd33glw2
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum + rusqlite dans `server/`, front SolidJS dans `web/`, `Makefile`, `README.md`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F03 et section « Administration et exploitation » (lignes « Premier démarrage » et « Serveurs et utilisateurs »). Avant toute connexion, un administrateur doit définir à quel serveur IMAP/SMTP le webmail se rattache.

## À faire
- Migration : table `server_profiles` (nom, hôte IMAP, port, mode TLS, hôte SMTP, port, mode TLS, domaines associés, actif).
- API d'administration distincte de la session de courrier : `/api/admin/v1/servers` (lister, créer, modifier, tester), protégée par un jeton `WEBMAIL_ADMIN_TOKEN` (en-tête `Authorization: Bearer`). Sans jeton configuré, l'API d'administration est fermée.
- Préréglage Infomaniak : `mail.infomaniak.com`, IMAP 993 en TLS implicite, SMTP 587 en STARTTLS.
- Préréglage « banc local » sans vérification de certificat, proposé seulement si `WEBMAIL_ALLOW_TEST_PROFILE=1`.
- Test de connexion : résultats séparés pour la résolution DNS, la négociation TLS et l'authentification (identifiants fournis pour le test, jamais stockés), pour IMAP et SMTP. Aucun courrier n'est envoyé.
- Une configuration qui échoue au test n'écrase pas la dernière configuration fonctionnelle.
- Page `/admin` côté front : état vide avec « Ajouter un serveur », formulaire centré, résultat du diagnostic. Les appels passent par `web/src/sync/`.

## Critères d'acceptation
- [ ] Sans serveur défini, l'état vide et « Ajouter un serveur » s'affichent.
- [ ] Le préréglage Infomaniak expose les deux protocoles sécurisés.
- [ ] Un hôte inexistant donne un diagnostic « résolution », un port fermé un diagnostic distinct, et le profil précédent reste en service.
- [ ] L'API d'administration refuse toute requête sans le bon jeton.
- [ ] Les journaux ne contiennent ni mot de passe ni jeton.

## Vérification
```
make check
```
Tests Rust : hôte `invalide.invalid`, port local fermé, conservation du profil précédent, refus sans jeton.

## Dépendances
« Mettre en place les tests ».