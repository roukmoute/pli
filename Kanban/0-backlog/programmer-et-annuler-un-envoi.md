---
id: kl1g5x2nu
title: Programmer et annuler un envoi
depends_on: kt6ats33l
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/` avec file d'envoi durable `outbox`, front SolidJS dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X07, lignes « Modèles et envoi différé » et « Règles de tri et envois différés », paragraphe « Envoi fiable ». Jalon Avancé : relis le code actuel de la file d'envoi avant de commencer.

## À faire
- Étendre `outbox` : échéance en UTC, fuseau affiché, états « en attente », « annulé », « remis au serveur ».
- Envoi programmé : choix de la date, de l'heure et du fuseau dans le composeur. L'ordonnanceur du serveur soumet à l'échéance et reprend après redémarrage en conservant échéance et état.
- Délai d'annulation configurable (réglage du compte, désactivé par défaut) : après un envoi immédiat, la mise en file est confirmée tout de suite et « Annuler » est proposé tant que la soumission est différée.
- Annulation : transition atomique en base qui départage l'annulation et le début de la soumission. Une annulation confirmée garantit qu'aucun message ne part ; une annulation trop tardive est refusée avec l'état réel.
- Vue « Envois en attente » : modifier (retour en brouillon) ou annuler avant la prise en charge.
- Une annulation rend le brouillon intact.

## Critères d'acceptation
- [ ] Un envoi programmé conserve son échéance après redémarrage et part à l'heure prévue.
- [ ] Une annulation confirmée avant la soumission empêche l'envoi (aucun message sur le banc).
- [ ] Course annulation contre départ, répétée 200 fois en test : jamais « annulé » et envoyé à la fois.
- [ ] Le délai d'annulation fonctionne sur un envoi immédiat.
- [ ] L'heure affichée respecte le fuseau choisi.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Traiter un envoi incertain ou partiellement refusé ».