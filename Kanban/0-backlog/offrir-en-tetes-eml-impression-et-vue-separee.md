---
id: kycyzji2x
title: Offrir en-têtes, EML, impression et vue séparée
depends_on: kx7bmitqo, k1x8jlzjn, ktzs898x5
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, fil de lecture, réponses et pièces de brouillon existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L08, ligne « Agir depuis un message ». Jalon Parité : relis le code actuel du menu « Plus » avant de commencer.

## À faire
- `GET /api/v1/messages/{id}/headers` : en-têtes bruts du message.
- `GET /api/v1/messages/{id}/eml` : message d'origine en flux (`BODY.PEEK[]`), servi en téléchargement `message/rfc822`.
- Menu « Plus » du message actif : Afficher les en-têtes, Télécharger l'EML, Imprimer, Transférer en pièce jointe, Ouvrir séparément.
- Impression : feuille de style d'impression du message assaini (objet, participants, date, corps), sans l'interface.
- Transférer en pièce jointe : nouveau brouillon avec le message joint en `.eml`.
- Ouverture séparée : route dédiée affichant le seul message, avec lien direct.
- Un libellé indique que l'action porte sur le message (et non le fil ou la sélection). À la fermeture, retour à la bonne ligne.

## Critères d'acceptation
- [ ] Les en-têtes et l'EML sont ceux du message actif du fil, pas du dernier.
- [ ] L'EML téléchargé est identique à l'original (empreinte).
- [ ] Le transfert en pièce jointe crée un brouillon avec un `.eml` lisible.
- [ ] Aucune de ces actions ne marque lu un message non ouvert, ni ne sert un message d'un autre compte.
- [ ] Fermer la vue séparée ou les en-têtes retrouve la bonne ligne.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Aperçu avant impression d'un message.

## Dépendances
« Ouvrir un message assaini et retrouver la liste », « Répondre, répondre à tous et transférer », « Joindre, annuler et reprendre un fichier ».