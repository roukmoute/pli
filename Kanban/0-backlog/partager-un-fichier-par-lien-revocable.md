---
id: knehtpjgz
title: Partager un fichier par lien révocable
depends_on: kol8sgbht
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust/Axum dans `server/`, table `files`, front SolidJS dans `web/`, composeur). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X02 (troisième partie), lignes « Fichiers et pièces jointes » et « Fichiers et liens de partage ». Dans le composeur, l'utilisateur choisit entre joindre une copie et insérer un lien de partage.

## À faire
- Migration `share_links` : jeton aléatoire long (stocké haché), fichier, version, droit d'accès, échéance, date de révocation, compte créateur.
- API : créer, lister, révoquer un lien pour un fichier du compte.
- Route publique `GET /p/{jeton}` : sert le fichier en flux si le lien est valide ; un lien expiré ou révoqué répond 410 sans indice sur le fichier. En-têtes de téléchargement sûrs, pas de mise en cache partagée.
- Composeur : « Insérer un lien de partage » depuis Mes fichiers, avec le choix de l'échéance. Affichage de qui peut l'ouvrir (toute personne ayant le lien), de l'échéance et de la révocation.
- Onglet « Mes fichiers » : liste des liens actifs d'un fichier et bouton Révoquer.
- Supprimer un fichier révoque ses liens.

## Critères d'acceptation
- [ ] Un lien valide télécharge le fichier sans session.
- [ ] Un lien expiré ou révoqué ne s'ouvre plus.
- [ ] Le jeton n'est pas stocké en clair et n'apparaît pas dans les journaux.
- [ ] Le composeur affiche l'échéance et la portée du lien inséré.
- [ ] Un jeton invalide ne révèle pas si un fichier existe.

## Vérification
```
make check
```

## Dépendances
« Gérer « Mes fichiers » et leur quota ».