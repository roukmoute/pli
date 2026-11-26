---
id: kequqz3li
title: "Régler les préférences de lecture et d'envoi"
depends_on: k5q2ki3ia, kk67sindc
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, dictionnaire `web/src/i18n/`, réglages d'identité et de rédaction existants, API d'administration protégée par jeton). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte A03, lignes « Identités et préférences » et « Apparence et fonctions du courrier ». Jalon Parité : relis le code actuel des réglages avant de commencer.

## À faire
- Réglages globaux dans l'administration : thème, langue, format horaire, largeur de lecture, densité, limites, et pour chacun si l'utilisateur peut le surcharger.
- Préférences par compte, côté serveur : langue, fuseau, format horaire, nombre de lignes par page, marquage lu à l'ouverture, lecture en série.
- Préférences par appareil (stockage local) : largeur de lecture et densité.
- Écran de préférences : pour chaque réglage, indiquer s'il est hérité du réglage global ou modifiable, et s'il prend effet tout de suite ou à la prochaine session.
- Ajouter une seconde langue (anglais) au dictionnaire pour prouver le changement de langue, y compris sur la page de connexion.
- Dates et heures affichées selon le fuseau et le format choisis.

## Critères d'acceptation
- [ ] Densité et largeur de lecture se retrouvent sur le même appareil après reconnexion ; langue et signature se retrouvent sur un autre appareil.
- [ ] Un réglage verrouillé par l'administrateur apparaît comme hérité et non modifiable.
- [ ] Changer le nombre de lignes par page change la taille de la première page servie.
- [ ] Aucun libellé ne reste en français quand l'anglais est choisi (test sur les clés).

## Vérification
```
make check
```

## Dépendances
« Configurer le profil serveur Infomaniak et son diagnostic », « Personnaliser identité, signature et format de rédaction ».