---
id: kjdx6zusm
title: Importer les en-têtes et trier les nouveaux expéditeurs
depends_on: k2j1ykpie
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, synchroniseur dans `server/src/sync/`, banc dans `bench/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F09, paragraphes « Premier rattachement » et « Synchroniser vite et juste », lignes « Message et indicateurs » et « Tri des expéditeurs » du tableau des données. Le Screener est actif dès le rattachement : tout message arrivé dans INBOX après le repère, d'un expéditeur sans décision, attend une décision.

## À faire
- Migration `messages` : clé (compte, dossier, UIDVALIDITY, UID) ; Message-ID (rapprochement, pas clé unique), expéditeur (adresse normalisée, nom), destinataires, objet, dates, indicateurs, taille, présence de pièces, extrait, `thread_id`.
- Migration `sender_decisions` : (compte, adresse) → état `en_attente`, `accepte` ou `refuse`, destination `reception`, `flux` ou `justificatifs`, révision.
- Import des en-têtes par lots de taille fixe (ENVELOPE, FLAGS, BODYSTRUCTURE, References), du plus récent au plus ancien, avec `BODY.PEEK` uniquement. La reprise au milieu d'un lot ne crée pas de doublon.
- Règle de tri : un message d'INBOX dont l'UID est au moins égal au repère et dont l'expéditeur n'a pas de décision crée une décision `en_attente` (une seule par adresse et par compte). Les messages antérieurs au repère sont admis en Réception sans créer de décision. Les autres dossiers ne passent pas par le tri.
- Fils : `thread_id` propre au compte, calculé par In-Reply-To et References. Deux messages de même objet sans lien restent distincts.
- Détection des arrivées par interrogation périodique d'INBOX (IDLE viendra plus tard), y compris pendant l'import.
- `GET /api/v1/screener` : expéditeurs en attente (adresse réelle, nom déclaré, dernier objet, extrait, nombre de messages).

## Critères d'acceptation
- [ ] Deux messages d'une adresse inconnue reçus pendant l'import donnent une seule entrée en attente, y compris après redémarrage.
- [ ] L'historique antérieur au repère est importé sans décision en attente.
- [ ] Aucun message n'est marqué lu sur le serveur IMAP par l'import.
- [ ] Une interruption au milieu d'un lot puis une reprise laissent une ligne par message.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Découvrir les dossiers et enregistrer le repère INBOX ».