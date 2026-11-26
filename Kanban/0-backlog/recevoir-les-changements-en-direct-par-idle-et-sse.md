---
id: kh8stj2yu
title: Recevoir les changements en direct par IDLE et SSE
depends_on: kpji7rvfl, kil5ghasf
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum/Tokio dans `server/`, synchroniseur dans `server/src/sync/`, front dans `web/`, banc dans `bench/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S02, paragraphes « Infomaniak » et « Contrat entre front et back ». Les arrivées sont aujourd'hui détectées par interrogation périodique et le navigateur n'est pas prévenu.

## À faire
- Synchroniseur : si le serveur annonce IDLE, surveiller INBOX par IDLE avec une seule connexion par compte, relancée avant l'expiration. Sinon, garder l'interrogation périodique. Les autres dossiers sont vérifiés à l'ouverture et périodiquement.
- Lire `bench/README.md` : si GreenMail n'annonce pas IDLE, ajouter au banc un conteneur Dovecot (image officielle, tag vérifié) avec les mêmes comptes, utilisé par les tests de cette carte, et le documenter.
- `GET /api/v1/events` : flux SSE Axum (`axum::response::sse`) privé au compte authentifié, qui annonce chaque nouvelle révision et les vues touchées, avec un battement régulier. Aucun contenu de message dans l'événement.
- `GET /api/v1/changes?depuis={revision}` : changements du compte depuis une révision (résumés ajoutés, modifiés, retirés, compteurs).
- Front : `web/src/sync/evenements.ts` ouvre le flux, récupère les changements et les applique par identifiant au store. Une seule connexion SSE partagée entre les onglets d'un même compte (BroadcastChannel ou SharedWorker).

## Critères d'acceptation
- [ ] Un message d'un expéditeur admis apparaît dans la bonne vue et incrémente son compteur sans rechargement.
- [ ] Un message d'un nouvel expéditeur n'incrémente que le compteur du Screener.
- [ ] Trois clients SSE du même compte n'ouvrent qu'une connexion de surveillance IMAP.
- [ ] Le flux d'un compte ne reçoit jamais la révision d'un autre.
- [ ] Sans IDLE, l'interrogation périodique produit les mêmes événements.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Servir la première page indexée par curseur ».