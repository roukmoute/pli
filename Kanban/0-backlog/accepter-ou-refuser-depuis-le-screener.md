---
id: kfi9t1244
title: Accepter ou refuser depuis le Screener
depends_on: kil5ghasf
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, module `web/src/sync/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F10, ligne « Nouveaux expéditeurs · Screener », ligne « Trier un expéditeur », paragraphe « Contrat entre front et back ». La table `sender_decisions` et `GET /api/v1/screener` existent ; il manque la décision et son écran.

## À faire
- `POST /api/v1/screener/decisions` : adresse, décision (`accepte` ou `refuse`), destination (`reception` par défaut, `flux`, `justificatifs`), révision attendue. Si la révision ne correspond pas, répondre 409 avec la décision enregistrée. Mise à jour et nouvelle révision du compte dans une seule transaction.
- La décision s'applique à tous les messages présents de l'adresse et aux prochaines arrivées du compte. Refuser ne déplace rien en IMAP et n'envoie aucun signalement : le dossier Indésirables reste intact.
- `GET /api/v1/screener/historique` : décisions prises, avec leurs messages. Revenir sur un refus ou changer de destination rétablit les messages encore présents.
- Front : page Screener ouverte par le bouton de la Réception. Une ligne par adresse (adresse réelle, nom déclaré, objet, extrait). « Accepter » (Réception par défaut, choix Flux ou Justificatifs) et « Refuser ». Historique du tri accessible depuis le menu de l'avatar.
- Action `accepterExpediteur` / `refuserExpediteur` dans `web/src/sync/`, optimiste : la ligne disparaît, le compteur baisse, les messages apparaissent dans la vue choisie ; un 409 affiche la décision enregistrée et permet de la modifier.

## Critères d'acceptation
- [ ] Accepter vers Réception, Flux ou Justificatifs rend les messages visibles dans cette vue seulement.
- [ ] Refuser une lettre d'information la retire des vues ordinaires, la laisse consultable dans l'historique et ne touche pas au dossier Indésirables.
- [ ] La décision vaut pour l'arrivée suivante de la même adresse et survit à un redémarrage.
- [ ] Deux décisions opposées concurrentes : la seconde reçoit la décision enregistrée.
- [ ] Les décisions d'un compte ne s'appliquent pas à un autre.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Comparer l'écran du Screener avec la planche de référence.

## Dépendances
« Afficher la Réception et son menu ».