---
id: kbnn0cnop
title: "Tester l'isolation entre comptes et la compatibilité"
depends_on: k7pfstynd, k9mxiyet6, k5nrtjmrj, kgv7oozd2, kycyzji2x, kqhbe3h9k, kol8sgbht, k7smvbih7
---

## Contexte
Webmail Infomaniak à interface HEY (tests Playwright dans `e2e/`, `make e2e`, banc dans `bench/`, plusieurs boîtes par utilisateur). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte A06 et paragraphe « Essais de fiabilité ». Ce test d'isolation précède l'ouverture à plusieurs utilisateurs.

## À faire
- Activer Firefox et WebKit dans Playwright en plus de Chromium, et deux tailles d'écran (390 × 844 et 1440 × 900). Cible `make e2e-compat`.
- Jeu de test : deux utilisateurs et deux boîtes dont des dossiers contiennent les mêmes UID.
- Sessions : un cookie de session d'un utilisateur ne donne rien de l'autre ; la déconnexion invalide la session côté serveur.
- Pour chaque type d'accès, tenter avec les identifiants de l'autre compte : liste, message, pièce, EML, recherche, lien profond, flux d'événements, brouillon, fichier, règle, téléchargement. Tous refusés.
- Images distantes : bloquées par défaut dans les trois navigateurs ; aucune requête tierce à l'ouverture d'un message.
- Caches : après changement de compte ou déconnexion, IndexedDB ne contient plus les données de l'ancien compte.
- Notifications et événements : un message pour A ne produit rien dans la session de B.
- Corriger les défauts mineurs ; toute fuite intercompte est signalée en tête du journal de carte comme bloquante.

## Critères d'acceptation
- [ ] `make e2e-compat` passe sur les trois navigateurs et les deux tailles.
- [ ] Chaque type d'accès de la liste a un test de refus.
- [ ] Aucune requête vers un domaine tiers n'est émise à la lecture d'un message piégé.
- [ ] Les écarts de compatibilité entre navigateurs sont listés dans `docs/recette/compatibilite.md`.

## Vérification
```
make e2e-compat
make check
```

## À vérifier par l'humain
- Un passage manuel sur Safari iOS et Chrome Android.

## Dépendances
« Tester le jalon Lecture de bout en bout avec Playwright », « Télécharger et prévisualiser une pièce reçue », « Lire et répondre sur téléphone », « Rattacher une deuxième boîte ».