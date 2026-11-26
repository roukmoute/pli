---
id: kklmrw0if
title: Composer un premier brouillon
depends_on: kil5ghasf
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, module `web/src/sync/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C01, paragraphes « Nouveau message » et « Brouillons », ligne « Brouillon et envoi ». Le bouton « Écrire » mène à une page provisoire.

## À faire
- Migration `drafts` : identifiant stable généré par le client à la première saisie, version, compte émetteur, identité, À, Cc, Cci, objet, corps (texte et HTML), références de pièces, lien de réponse éventuel.
- API : `PUT /api/v1/drafts/{id}` avec version attendue, `GET`, `DELETE`, liste. À l'enregistrement, le synchroniseur dépose le brouillon dans le dossier IMAP Brouillons (`APPEND`, en remplaçant la version précédente).
- Page `/ecrire` dans la colonne principale, focus initial sur le destinataire. Champs À, Cc, Cci en étiquettes : saisie d'une adresse nouvelle, collage de plusieurs adresses avec noms accentués, correction individuelle d'une adresse invalide, signalement d'un doublon.
- Objet et corps. Éditeur de texte et HTML chargé à la première utilisation (choisir une bibliothèque légère compatible Solid après vérification, ou `contenteditable` maîtrisé) ; listes, liens et citations conservés après fermeture et réouverture. Le HTML produit est assaini côté serveur.
- Sauvegarde manuelle par Ctrl ou Cmd + S ; état affiché « synchronisation en cours » puis « enregistré dans Brouillons ». Fermer propose de conserver ou d'abandonner.
- Vue Brouillons depuis le menu : un clic ou un double-clic reprend une seule session de rédaction.

## Critères d'acceptation
- [ ] Créer un message avec À, Cc et Cci, corriger une adresse invalide, sauvegarder, fermer, rouvrir : objet, destinataires et contenu sont intacts.
- [ ] Coller `"Zoé Müller" <zoe@exemple.test>, bob@exemple.test` crée deux destinataires.
- [ ] Le brouillon existe une seule fois dans le dossier Brouillons IMAP après trois sauvegardes.
- [ ] Une version attendue périmée est refusée sans écraser.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Afficher la Réception et son menu ».