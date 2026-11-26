---
id: kwg66u0bh
title: "Travailler au clavier et au lecteur d'écran"
depends_on: k7pfstynd, ks5rethb9, kqhbe3h9k, k5nrtjmrj, kvogo9if0, kdbgqguzb
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, tests Playwright dans `e2e/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte A02, paragraphes « Clavier » et « Accessibilité ».

## À faire
- Gestionnaire de raccourcis central (`web/src/clavier/`), inactif dans les champs de saisie et les zones éditables : H menu, S ou / recherche, W ou C composer, J et K lignes, X sélectionner, Entrée ouvrir ; dans un fil, R répondre, L « Répondre plus tard », A « De côté », M actions supplémentaires ; flèches pour le focus ; Échap revient au déclencheur ; Ctrl ou Cmd + S sauvegarde le brouillon ; Ctrl ou Cmd + Entrée envoie seulement si le réglage est activé.
- Les raccourcis dont la fonction n'existe pas encore sont déclarés mais sans effet, sans erreur.
- Chaque icône d'action a un nom accessible et un focus visible. Les boîtes de dialogue retiennent le focus et le rendent au déclencheur.
- Zones `aria-live` : arrivée de courrier, résultat d'une action, compteurs, sans relire toute la liste.
- Erreurs, opérations en attente et compteurs compréhensibles sans la couleur.
- Tests Playwright avec `@axe-core/playwright` (vérifier la version) sur connexion, Réception, menu, Screener, fil, composeur, recherche.
- Aide des raccourcis affichable par `?`.

## Critères d'acceptation
- [ ] Le parcours connexion, recherche, lecture, réponse et archivage se fait entièrement au clavier (test Playwright).
- [ ] Taper « réunion » dans le corps d'un message saisit le mot entier ; S et R tapés dans un champ restent du texte.
- [ ] Aucune violation axe sérieuse ou critique sur les écrans testés.
- [ ] Échap ferme chaque surimpression et rend le focus au déclencheur.

## Vérification
```
make check
make e2e
```

## À vérifier par l'humain
- Un passage avec un vrai lecteur d'écran (NVDA, VoiceOver ou Orca) sur la Réception et un fil.

## Dépendances
« Tester le jalon Lecture de bout en bout avec Playwright », « Archiver une sélection et garder la place de lecture », « Chercher dans le dossier actif avec FTS5 », « Lire et répondre sur téléphone ».