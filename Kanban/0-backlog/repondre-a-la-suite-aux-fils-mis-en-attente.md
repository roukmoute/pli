---
id: khy2umjkg
title: Répondre à la suite aux fils mis en attente
depends_on: kvogo9if0, kq9cyz1p6, kt6ats33l
---

## Contexte
Webmail Infomaniak à interface HEY (front SolidJS dans `web/`, pile « Répondre plus tard », réponses, brouillons et états d'envoi existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C10, ligne « Piles au bas de la réception » et paragraphe « Réponse et transfert ». Jalon Parité : relis le code actuel des piles et du composeur avant de commencer.

## À faire
- Commande « Répondre à la suite » depuis la pile « Répondre plus tard » : ouvre un mode qui présente les fils un par un, chacun avec son message à gauche ou au-dessus et sa réponse.
- Chaque fil garde ses propres destinataires et son propre brouillon (identifiant distinct).
- Commandes par fil : Envoyer et passer au suivant, Ignorer, Quitter.
- Un fil sort de la pile seulement quand la soumission SMTP de sa réponse est confirmée. Une annulation, un refus ou un envoi « à vérifier » laisse le fil dans la pile.
- Quitter au milieu conserve les brouillons commencés ; reprendre le mode retrouve chaque brouillon sur son fil.
- Parcours utilisable au clavier, avec annonce de la position (« 2 sur 3 »).

## Critères d'acceptation
- [ ] Pile de trois fils : répondre au premier, ignorer le deuxième, interrompre le troisième en brouillon. Seul le premier sort de la pile, après acceptation SMTP.
- [ ] Chaque fil conserve ses destinataires après reprise.
- [ ] Un envoi refusé ou incertain laisse le fil retrouvable dans la pile.
- [ ] Aucun brouillon n'est mélangé entre deux fils.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Gérer les piles « Répondre plus tard » et « De côté » », « Passer de la réponse rapide à la rédaction complète », « Traiter un envoi incertain ou partiellement refusé ».