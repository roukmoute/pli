---
id: kwpaqe22g
title: Prioriser la lecture et le préchargement
depends_on: k9g91mi7z, k9mxiyet6
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, synchroniseur dans `server/src/sync/`, front dans `web/`, cache IndexedDB). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S05, paragraphe « Priorité réseau ». Aujourd'hui, l'import de l'historique et la lecture demandée par l'utilisateur se disputent la connexion IMAP.

## À faire
- Synchroniseur : file de travaux à trois priorités par compte : lecture demandée par l'utilisateur, puis préchargement, puis indexation de l'historique. Une lecture passe devant un lot d'indexation en cours.
- Regrouper les demandes identiques déjà en cours (même corps demandé deux fois = une seule récupération).
- Annuler les travaux devenus inutiles (préchargement d'un dossier quitté).
- `GET /api/v1/messages/{id}?prechargement=1` : récupère le corps sans marquer lu, refuse au-delà d'une petite taille.
- Front : au repos, précharger le corps du message voisin lors d'une lecture au clavier, et le ranger dans le cache. Annuler sur changement rapide de dossier, en mode économie de données (`navigator.connection.saveData`) ou réseau contraint.
- Les pièces lourdes ne sont jamais préchargées.

## Critères d'acceptation
- [ ] Pendant un import de 10 000 messages, une demande de lecture est servie avant la fin du lot suivant.
- [ ] Un corps préchargé reste non lu, côté webmail et côté IMAP.
- [ ] Deux demandes simultanées du même corps donnent une seule récupération IMAP.
- [ ] En mode économie de données, aucun préchargement ne part.
- [ ] Une pièce lourde attend une action de l'utilisateur.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Mettre vues et fils récents en cache IndexedDB », « Télécharger et prévisualiser une pièce reçue ».