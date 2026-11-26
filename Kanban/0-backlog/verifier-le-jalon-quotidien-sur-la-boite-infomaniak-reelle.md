---
id: kcxddfy5d
title: Vérifier le jalon Quotidien sur la boîte Infomaniak réelle
depends_on: k7smvbih7, kpmdfz4yx, kp6ohvyka
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, commande `make recette-lecture` et rapport `docs/recette/infomaniak-lecture.md` déjà en place). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, section « Jalons et critères de livraison » (jalon Quotidien). Classement, envoi et règles ont été vérifiés sur un banc local et une fausse API ; cette carte les confronte à la vraie boîte d'essai.

## Prérequis fourni par l'humain
`.env.recette` (ignoré par git) contient `RECETTE_IMAP_USER`, `RECETTE_IMAP_PASSWORD`, `RECETTE_SMTP_TO` (adresse qui accepte les envois de test), `INFOMANIAK_API_TOKEN`, `INFOMANIAK_MAIL_HOSTING_ID`, `INFOMANIAK_MAILBOX_NAME`. **Si une valeur manque, arrête-toi et signale-le dans le journal.** N'écris jamais ces valeurs dans un fichier suivi, un journal ou une sortie.

## Limites strictes
- Envois : uniquement vers `RECETTE_SMTP_TO` et vers la boîte elle-même, cinq messages au plus.
- Écritures IMAP : uniquement sur des messages envoyés par ce test et dans un dossier `webmail-recette` créé pour l'occasion.
- Règles : uniquement des règles dont le nom commence par `webmail-recette`, supprimées à la fin.
- Ne supprime ni ne déplace aucun message préexistant.

## À faire
- `make recette-quotidien` : envoi d'un message avec pièce à soi-même, réception, archivage, restauration, déplacement vers `webmail-recette`, réponse vers `RECETTE_SMTP_TO`.
- Vérifier qu'il n'y a qu'une seule copie dans Envoyés (Infomaniak peut en créer une lui-même à la soumission : dans ce cas, adapter la copie par `APPEND` et le documenter).
- Règles : créer, réordonner, désactiver, relire ; vérifier qu'un message témoin est classé par le serveur.
- Relever IDLE réel (délai de notification), MOVE, UIDPLUS, et les droits accordés au jeton.
- Écrire `docs/recette/infomaniak-quotidien.md` sans adresse, objet ni contenu. Nettoyer dossier et règles de test.

## Critères d'acceptation
- [ ] La commande se termine avec un résultat par étape et laisse la boîte nettoyée.
- [ ] Les limites ci-dessus sont respectées (vérifiable dans le code de la commande).
- [ ] Le rapport liste les écarts avec le banc et les corrections faites.
- [ ] Aucun secret dans `git status`.

## Vérification
```
make recette-quotidien
make check
git status --short
```

## À vérifier par l'humain
- Depuis un autre client mail : le message est bien arrivé sur l'adresse de test et la règle a bien classé le message témoin.

## Dépendances
« Gérer les règles de tri par l'API Infomaniak », « Tester le classement de bout en bout », « Tester la rédaction et l'envoi de bout en bout ».