---
id: kut392580
title: Lire le Flux et retrouver un justificatif
depends_on: kfi9t1244, kbissnj87, kirtkkako
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L09, ligne « Flux · The Feed et Justificatifs · Paper Trail », paragraphe « Dossiers et conversations ». Les vues Flux et Justificatifs affichent pour l'instant la liste générique.

## À faire
- Flux : lettres d'information en cartes de lecture, les plus récentes d'abord, avec un aperçu du corps assaini. Charger les cartes par lots et les aperçus à l'approche de la zone visible ; une image lente garde sa place.
- Justificatifs : liste chronologique compacte (expéditeur, objet, date).
- Dans les deux vues : rejoindre le message d'origine dans son fil, et changer sa destination en un geste.
- Destination ponctuelle : migration ajoutant une destination par message, qui prime sur celle de l'expéditeur. Opération `changer_destination` (portée message).
- Changer la destination des prochains messages de l'expéditeur demande un choix explicite distinct, qui met à jour la décision du tri avec contrôle de révision.
- L'API des vues tient compte de la destination ponctuelle dans les listes et compteurs.

## Critères d'acceptation
- [ ] Une lettre d'information assignée au Flux se lit dans une carte.
- [ ] Un reçu se retrouve dans la liste chronologique des Justificatifs.
- [ ] Déplacer ce seul reçu vers la Réception ne change pas la destination des prochains messages du même expéditeur.
- [ ] Le choix explicite « pour les prochains messages » change bien la décision enregistrée.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Rendu des cartes du Flux et de la liste des Justificatifs.

## Dépendances
« Accepter ou refuser depuis le Screener », « Paginer et restaurer la position », « Traiter une action optimiste : étoile, lu, non lu ».