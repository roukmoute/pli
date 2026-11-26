---
id: k5nrtjmrj
title: Lire et répondre sur téléphone
depends_on: kil5ghasf, k7pfstynd, k1x8jlzjn
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS + CSS Modules dans `web/`, tests Playwright dans `e2e/`, planche dans `docs/reference/planche.md`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte A01, paragraphe « Téléphone et tablette ». L'interface a été construite d'abord pour le bureau.

## À faire
- Mise en page adaptative aux formats 390 × 844, 768 × 1024 et 1440 × 900, et au zoom à 200 %.
- Téléphone : recherche et profil en haut, « Écrire » à côté du titre, menu principal ouvert depuis le bas de l'écran (panneau glissant) ; réception, fil et rédaction s'ouvrent successivement en plein écran.
- Tablette : largeur de lecture adaptée à l'espace utile.
- Action Retour (bouton et historique du navigateur) : retrouve le fil, la section et la ligne d'origine.
- Clavier virtuel : destinataires et bouton d'envoi restent accessibles (`visualViewport`, unités `dvh`).
- Cibles tactiles d'au moins 44 px ; aucune action dépendante du survol.
- Lien profond : rétablit le compte, la vue et le message après contrôle des droits ; sans session, passe par la connexion puis revient au lien.
- Tests Playwright en fenêtres mobiles et tactiles : menu, ouverture d'un fil, Répondre, Retour.

## Critères d'acceptation
- [ ] À 390 × 844 : le menu ouvre la Réception, toucher l'objet ouvre le fil, toucher Répondre lance l'éditeur, Retour restitue la section et le rang.
- [ ] Aucun défilement horizontal aux trois formats ni à 200 % de zoom.
- [ ] Un lien profond vers un message d'un autre compte est refusé.
- [ ] Les tests mobiles passent dans `make e2e`.

## Vérification
```
make check
make e2e
```

## À vérifier par l'humain
- Sur un vrai téléphone : clavier virtuel, menu depuis le bas, geste Retour, comparaison avec les captures mobiles de la planche.

## Dépendances
« Afficher la Réception et son menu », « Tester le jalon Lecture de bout en bout avec Playwright », « Répondre, répondre à tous et transférer ».