---
id: kqgzqscrh
title: Organiser les dossiers et traiter les indésirables
depends_on: kyf0sqqpb
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, sélecteur de dossiers et opérations existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L06, paragraphe « Dossiers et conversations », ligne « Tri choisi et antispam » du premier tableau.

## À faire
- API et synchroniseur : créer, renommer, déplacer un dossier (`CREATE`, `RENAME`), le masquer ou le réafficher (réglage local). Les dossiers à rôle ne se renomment pas.
- Front : gestion de l'arborescence depuis le menu ; le sélecteur de destination se met à jour aussitôt.
- Vue « Favoris » : filtre des messages étoilés du compte.
- Opérations `vers_indesirables` et `sortir_des_indesirables`. Le libellé de la commande est exactement « Déplacer vers Indésirables » : c'est un simple déplacement IMAP. Le libellé « Signaler à Infomaniak » n'est pas utilisé.
- Le refus d'un expéditeur au Screener reste indépendant du dossier Indésirables : aucun des deux ne modifie l'autre.

## Critères d'acceptation
- [ ] Créer puis renommer un sous-dossier actualise le sélecteur de destination sans rechargement.
- [ ] Déplacer vers Indésirables puis restaurer met à jour les deux compteurs.
- [ ] Refuser un expéditeur ne déplace aucun message vers Indésirables ; déplacer un message vers Indésirables ne crée aucune décision de tri.
- [ ] Un dossier masqué disparaît du menu et des choix de classement, puis revient quand on le réaffiche.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Déplacer et copier par le menu et le glisser-déposer ».