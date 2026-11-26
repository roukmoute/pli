---
id: kol8sgbht
title: Gérer « Mes fichiers » et leur quota
depends_on: ktzs898x5, kdbgqguzb, kwwlmwlbk
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, stockage dans `WEBMAIL_DATA_DIR/files`, front SolidJS dans `web/`, page Fichiers et pièces de brouillon existantes). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X02 (deuxième partie), lignes « Fichiers et pièces jointes » et « Fichiers et liens de partage ».

## À faire
- Migration `files` : identifiant stable, compte, dossier parent, nom, version, taille imputée au quota.
- API `/api/v1/files` : importer (en flux), parcourir, renommer, déplacer, supprimer, télécharger.
- Quota par compte, réglable par `WEBMAIL_FILES_QUOTA_MB` (défaut 1024). Un dépassement renvoie l'espace nécessaire et l'espace libre.
- Onglet « Mes fichiers » de la page Fichiers.
- « Conserver une copie dans Mes fichiers » depuis le lecteur de pièces et depuis la galerie.
- Composeur : « Joindre depuis Mes fichiers » crée une copie figée pour le brouillon. Renommer ou supprimer l'original ne change pas la pièce jointe.

## Critères d'acceptation
- [ ] Importer, renommer, déplacer, télécharger puis supprimer un fichier fonctionne et met à jour l'espace utilisé.
- [ ] Sauvegarder une pièce reçue dans Mes fichiers, la joindre à un brouillon, renommer puis supprimer l'original : la pièce envoyée a le contenu d'origine.
- [ ] Quota atteint pendant l'ajout d'une pièce : le brouillon est préservé et l'espace nécessaire est indiqué.
- [ ] Un fichier d'un compte est inaccessible depuis un autre.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Joindre, annuler et reprendre un fichier », « Soumettre une seule fois et classer dans Envoyés », « Afficher la galerie « Pièces du courrier » ».