---
id: kd8focfhy
title: Gérer les limites de stockage et les reprises
depends_on: k9g91mi7z, kd9c3y4qa
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, cache IndexedDB dans `web/src/sync/cache.ts`, brouillons locaux). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte S06, tableau « Caches et réactivité » et paragraphe « Reprise après incident ».

## À faire
- Budget de volume pour le cache local, suivi avec `navigator.storage.estimate`.
- Éviction ordonnée : lectures anciennes d'abord, puis listes ; jamais les brouillons non synchronisés ni les opérations en attente.
- Stockage plein (`QuotaExceededError`) : informer l'utilisateur, libérer les lectures, et proposer d'enregistrer le brouillon en ligne.
- Stockage évincé par le navigateur : le détecter à l'ouverture et repartir du serveur sans erreur.
- Mode « appareil partagé » (connexion sans « appareil de confiance ») : traces persistantes limitées ; à la déconnexion ou à la fermeture avec un brouillon local, proposer de l'enregistrer en ligne.
- Hors ligne : un message jamais ouvert affiche « contenu disponible à la reconnexion ».
- Une opération en attente devenue impossible (déplacement vers un dossier supprimé) est expliquée et laissée à la décision de l'utilisateur.

## Critères d'acceptation
- [ ] Budget dépassé : les lectures anciennes partent, le brouillon local reste.
- [ ] Quota plein simulé : message compréhensible, aucune perte de texte.
- [ ] Base IndexedDB supprimée entre deux ouvertures : l'application redémarre proprement.
- [ ] En mode partagé, rien de personnel ne reste après déconnexion.
- [ ] Hors ligne, un message non mis en cache affiche l'état prévu.

## Vérification
```
make check
```

## Dépendances
« Mettre vues et fils récents en cache IndexedDB », « Retrouver un brouillon après une coupure ».