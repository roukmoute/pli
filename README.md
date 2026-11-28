# Pli — le webmail réinventé

Un proche attend votre réponse. Votre lettre préférée peut attendre dimanche. Ce reçu vous sera surtout utile dans six mois.

Pli vous laisse choisir où retrouver chacun de ces courriers. La première fois qu’un expéditeur vous écrit, vous voyez son nom, son adresse et un extrait avant de décider où ranger ses messages.

| Vue | Ce que vous y gardez |
| --- | --- |
| **Réception** | Les conversations que vous voulez suivre de près. |
| **Flux** | Les lettres et les nouvelles à parcourir quand vous en avez le temps. |
| **Justificatifs** | Les reçus et les confirmations à retrouver le moment venu. |

Votre choix vaut pour les prochains messages de cette adresse et pour ceux qui se trouvent déjà dans Réception. Depuis l’historique du tri, vous pouvez revenir sur un refus ou changer de destination.

![La Réception de Pli avec l’objet et un extrait du dernier message de chaque fil.](docs/images/reception.png)

## Reprendre là où vous en étiez

Ouvrez une conversation pour lire la dernière réponse. Un clic sur le nom d’un correspondant déplie un message précédent. Les images hébergées ailleurs s’affichent si vous le choisissez pour ce message.

Au retour en Réception, la liste déjà chargée s’affiche pendant que les nouveautés arrivent en arrière-plan. Vous retrouvez votre place même si le fil que vous venez de lire a rejoint les « Déjà vus ». Au clavier, **H** ouvre le menu et **Échap** le referme.

La liste se prolonge quand vous approchez du bas, ou avec « Afficher les fils suivants ». Elle garde toutes les pages déjà parcourues. Après un détour par un autre dossier, vous retrouvez la même ligne, même loin dans la boîte. Si un courrier arrive pendant que vous lisez plus bas, la ligne sous votre pointeur reste en place et « Nouveautés » vous ramène en haut quand vous le voulez.

L’étoile, « Marquer comme lu » et « Marquer comme non lu » s’appliquent dès votre geste. Pli transmet ensuite le changement à votre messagerie. Si la confirmation tarde, la ligne indique « En attente » et l’étoile s’entoure d’un pointillé. Si la messagerie refuse, la liste retrouve son état et vous le signale.

## Essayer Pli

Depuis la racine du projet, lancez l’interface de développement et ouvrez [Pli sur votre machine](http://127.0.0.1:5173).

```sh
make dev
```

Le guide détaille les prérequis, la configuration d’une boîte et les commandes de vérification. Vous pouvez aussi découvrir Pli avec une boîte de démonstration remplie de messages fictifs. [Guide technique](docs/guide-technique.md) · [Boîte de démonstration](bench/README.md)
