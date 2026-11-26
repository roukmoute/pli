---
id: kq9cyz1p6
title: Passer de la réponse rapide à la rédaction complète
depends_on: kd9c3y4qa, k1x8jlzjn, ktzs898x5
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, composeur, réponse rapide dans le fil, brouillons locaux et serveur). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C04, paragraphe « Réponse et transfert ».

## À faire
- La réponse rapide et le composeur complet partagent un seul état de brouillon (même identifiant) dans `web/src/sync/`.
- Commande « Agrandir » depuis la réponse rapide : ouvre le composeur complet avec le texte saisi, la position du curseur, l'identité, les destinataires et les pièces.
- Minimiser puis reprendre conserve le même brouillon.
- Le retour depuis le composeur rétablit le fil et le message d'origine.
- Garantir qu'aucun passage ne crée un second brouillon côté serveur ou dans Brouillons IMAP.

## Critères d'acceptation
- [ ] Texte saisi sous le message, agrandissement : texte, identité et destinataires identiques, même identifiant de brouillon.
- [ ] Agrandir, minimiser, consulter un autre fil, reprendre : rien n'est perdu.
- [ ] Après tout le parcours, un seul brouillon existe côté serveur.
- [ ] Le retour ouvre le fil d'origine.

## Vérification
```
make check
```

## Dépendances
« Retrouver un brouillon après une coupure », « Répondre, répondre à tous et transférer ».