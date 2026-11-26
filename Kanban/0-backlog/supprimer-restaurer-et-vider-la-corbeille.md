---
id: kmmb7jat5
title: Supprimer, restaurer et vider la corbeille
depends_on: ks5rethb9
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, opérations via `POST /api/v1/operations`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L05, ligne « Archiver, signaler ou supprimer ».

## À faire
- Type d'opération `corbeille` : le premier geste déplace vers le dossier de rôle Corbeille. Mémoriser le dossier d'origine du message.
- Type `restaurer` : depuis la Corbeille, retour au dossier d'origine s'il existe encore, sinon choix du dossier.
- Type `supprimer_definitivement` (depuis la Corbeille) : `\Deleted` puis `EXPUNGE` ciblé sur les UID concernés.
- Type `vider_dossier` pour Corbeille et Indésirables.
- Front : commande Supprimer dans le fil et la barre de sélection ; dans la Corbeille, Restaurer et Supprimer définitivement.
- Toute action définitive passe par une boîte de dialogue qui cite la portée exacte : nom du dossier et nombre d'éléments. Le focus y reste, Échap annule et rend le focus au déclencheur.

## Critères d'acceptation
- [ ] Supprimer déplace en Corbeille sans confirmation et de façon optimiste.
- [ ] Restaurer remet le message dans son dossier d'origine et ajuste les deux compteurs.
- [ ] Vider la Corbeille affiche son nom et le nombre d'éléments, et ne supprime rien si on annule.
- [ ] Une suppression définitive n'expurge que les messages ciblés.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Archiver une sélection et garder la place de lecture ».