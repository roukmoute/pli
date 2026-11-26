---
id: k7smvbih7
title: "Gérer les règles de tri par l'API Infomaniak"
depends_on: k5q2ki3ia, kfi9t1244, kyf0sqqpb
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X04, lignes « Règles de tri à la réception » et « Règles de tri et envois différés ». Les règles vivent chez Infomaniak et s'appliquent navigateur fermé. Lis la documentation de l'API des filtres sur developer.infomaniak.com (liens dans le cahier : ajout d'un filtre, réordonnancement) avant de coder.

## À faire
- Client `server/src/infomaniak_api/` : URL de base configurable, jeton d'API par compte stocké chiffré, identifiants de l'hébergement mail et de la boîte. Opérations : lister, créer, modifier, activer, désactiver, supprimer, réordonner les filtres.
- Fausse API Infomaniak locale pour les tests (serveur de test Rust), fidèle aux formats de la documentation, capable de simuler un refus de droits.
- Copie de lecture par boîte dans SQLite, rafraîchie après chaque écriture ou refus : l'état affiché est toujours celui relu chez l'hébergeur.
- API du webmail `/api/v1/rules` pour le compte actif.
- Front : écran « Règles » depuis le menu de l'avatar. Créer une règle sur l'expéditeur, le destinataire ou l'objet ; une ou toutes les conditions ; action et dossier (sélecteur de dossiers) ; modifier, activer, désactiver, réordonner.
- Droit manquant : nommer le droit que l'administrateur de la boîte peut accorder. Jeton absent : expliquer où le renseigner.
- Vérifier que les messages classés par une règle dans un dossier IMAP ne passent pas par le Screener (seul INBOX y passe).

## Critères d'acceptation
- [ ] Créer deux règles, les réordonner, en désactiver une : l'ordre et l'état relus concordent avec la fausse API.
- [ ] Un refus de droits est expliqué et l'affichage reste celui de l'hébergeur.
- [ ] Un message déposé dans un dossier par une règle garde son classement et ne crée pas d'entrée au Screener.
- [ ] Le jeton d'API n'est ni en clair en base ni dans les journaux.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Configurer le profil serveur Infomaniak et son diagnostic », « Accepter ou refuser depuis le Screener », « Déplacer et copier par le menu et le glisser-déposer ».