---
id: kyf0sqqpb
title: Déplacer et copier par le menu et le glisser-déposer
depends_on: ks5rethb9
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, opérations via `POST /api/v1/operations`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte L04, ligne « Déplacer ou copier ». L'archivage existe ; il faut le déplacement et la copie vers un dossier choisi.

## À faire
- Types d'opération `deplacer` et `copier` avec dossier cible, en réutilisant le chemin IMAP de l'archivage. Une copie attribue un nouvel UID, rattaché quand le serveur le communique.
- Composant « sélecteur de dossiers » réutilisable : dossiers du compte actif, filtrable au clavier.
- Menu « Plus » du fil et barre de sélection : Déplacer, Copier, puis choix du dossier.
- Bureau : glisser une ligne ou la sélection vers le sélecteur de dossiers ; Ctrl ou Cmd pendant le glisser demande une copie, avec un indice visible.
- Optimisme : lignes et compteurs ajustés tout de suite ; refus restauré élément par élément.
- Dossier cible supprimé entre-temps : message explicite, l'utilisateur choisit une autre destination.

## Critères d'acceptation
- [ ] Le menu et le glisser-déposer agissent sur le même lot et produisent la même opération.
- [ ] Une copie laisse l'original en place ; le message existe dans les deux dossiers côté IMAP.
- [ ] Un dossier cible absent ou un échec partiel donne un retour précis par élément.
- [ ] Le sélecteur ne montre que les dossiers du compte actif.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Le glisser-déposer et la touche de copie à la souris.

## Dépendances
« Archiver une sélection et garder la place de lecture ».