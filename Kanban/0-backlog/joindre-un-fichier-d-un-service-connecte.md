---
id: kpwtj6ty1
title: "Joindre un fichier d'un service connecté"
depends_on: kol8sgbht
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, « Mes fichiers » et pièces de brouillon existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X10, ligne « Services externes et connexion fédérée ». Le service retenu est **kDrive** d'Infomaniak. Lis sa documentation d'API sur developer.infomaniak.com avant de coder. Jalon Avancé.

## À faire
- Abstraction `ServiceDeFichiers` côté serveur (lister un dossier, télécharger un fichier), avec une première implémentation kDrive.
- Liaison par compte : autorisation du service (jeton stocké chiffré), avec affichage du compte lié, des autorisations accordées et de l'état après révocation.
- Fausse API kDrive locale pour les tests, y compris la révocation et l'indisponibilité.
- Composeur : « Joindre depuis kDrive » ouvre un sélecteur ; le fichier choisi est copié comme pièce figée du brouillon, avec progression.
- Révocation : toute nouvelle sélection est empêchée ; le texte et les pièces déjà copiées d'un brouillon ouvert sont préservés.
- Service indisponible : la lecture du courrier et la rédaction restent possibles ; seule la commande kDrive signale l'erreur.
- Une intégration désactivée par l'administrateur disparaît du composeur et du chargement initial.

## Critères d'acceptation
- [ ] Autoriser le service pour un compte, puis joindre un fichier à un message de ce compte.
- [ ] Révoquer l'accès empêche toute nouvelle sélection et préserve le brouillon ouvert.
- [ ] Panne simulée du service : lecture et rédaction fonctionnent.
- [ ] La liaison d'un compte n'est pas utilisable depuis un autre.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Avec un vrai compte kDrive : autoriser, joindre un fichier, révoquer.

## Dépendances
« Gérer « Mes fichiers » et leur quota ».