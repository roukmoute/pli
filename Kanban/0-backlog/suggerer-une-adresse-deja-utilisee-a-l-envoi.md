---
id: ktp72d144
title: "Suggérer une adresse déjà utilisée à l'envoi"
depends_on: kdbgqguzb
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, composeur et file d'envoi existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte R03, paragraphe « Nouveau message ». Les suggestions proviennent uniquement des destinataires auxquels ce compte a déjà envoyé un message.

## À faire
- Migration `sent_recipients` : (compte, adresse, nom, dernier envoi, nombre d'envois).
- Alimentation à l'acceptation SMTP uniquement, pour les destinataires acceptés par le serveur.
- `GET /api/v1/recipients/suggest?q=` : préfixe sur le nom ou l'adresse, tri par fréquence et date, limité au compte actif.
- Front : liste de suggestions sous les champs À, Cc et Cci, navigable au clavier (flèches, Entrée, Échap), avec les rôles ARIA d'une liste déroulante.
- La saisie d'une adresse inédite reste toujours possible.

## Critères d'acceptation
- [ ] Une adresse acceptée lors d'un envoi est suggérée dans À, Cc et Cci pour le même compte.
- [ ] Une adresse refusée par SMTP n'est pas suggérée.
- [ ] L'historique d'une autre boîte n'apparaît jamais.
- [ ] Les expéditeurs reçus mais jamais contactés ne sont pas suggérés.
- [ ] On peut saisir un destinataire inédit sans passer par la liste.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Soumettre une seule fois et classer dans Envoyés ».