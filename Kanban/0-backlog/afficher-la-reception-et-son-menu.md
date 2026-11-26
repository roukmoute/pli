---
id: kil5ghasf
title: Afficher la Réception et son menu
depends_on: k0ym6scsi, kpji7rvfl
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS + TypeScript strict + CSS Modules dans `web/`, charte dans `web/src/styles/tokens.css`, libellés dans `web/src/i18n/`, planche dans `docs/reference/planche.md`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F07, paragraphe « Bureau » et tableau des écrans. L'API `GET /api/v1/views/reception` existe. Règle du projet : les composants ne parlent qu'à `web/src/sync/`.

## À faire
- Routeur (bibliothèque de routage Solid compatible avec la version installée, à vérifier) : `/reception`, `/flux`, `/justificatifs`, `/dossier/:id`, `/ecrire` (page provisoire).
- Store Solid en mémoire dans `web/src/sync/` : vues chargées, section, ancre de défilement et ligne au focus, par vue. Action `chargerVue`.
- En-tête fixe : loupe de recherche en haut à gauche (bouton provisoire), bouton du menu centré, avatar du compte actif à droite.
- Réception : colonne centrée, titre visible, bouton « Écrire » en haut à droite, sections « Nouveaux » puis « Déjà vus ». Chaque ligne : avatar ou initiales, objet, expéditeur, extrait, date alignée à droite, repères d'étoile, de pièce, de réponse, de transfert et nombre de messages. État vide.
- Bouton près du titre indiquant le nombre d'expéditeurs en attente, visible même si la liste est vide.
- Menu en surimpression : champ pour filtrer les destinations, six tuiles (Réception, Flux, Justificatifs, Répondre plus tard, De côté, Fichiers), puis Brouillons, Envoyés, Archives, Indésirables, Corbeille et dossiers personnels. Échap ferme et rend le focus au bouton.
- Les vues Flux, Justificatifs et dossier réutilisent provisoirement la même liste.

## Critères d'acceptation
- [ ] Aller dans un dossier puis revenir à Réception redessine la liste depuis la mémoire, sans requête réseau bloquante (test avec `fetch` espionné), avec section, position et focus conservés.
- [ ] Changer la largeur de fenêtre conserve la section et le focus.
- [ ] Le compteur d'expéditeurs en attente s'affiche avec une Réception vide.
- [ ] Le menu se parcourt au clavier et chaque icône a un nom accessible.

## Vérification
```
make check
```

## À vérifier par l'humain
- Comparer la Réception vide, remplie et le menu ouvert avec `docs/reference/planche.md` aux trois formats d'écran.

## Dépendances
« Fixer la charte, les libellés et les écrans de référence », « Servir la première page indexée par curseur ».