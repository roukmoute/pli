---
id: kdbgqguzb
title: Soumettre une seule fois et classer dans Envoyés
depends_on: kd9c3y4qa, ktzs898x5
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, brouillons et pièces existants, banc SMTP dans `bench/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C07, paragraphe « Envoi fiable », ligne « Brouillon et envoi ». Le double envoi est une anomalie bloquante de livraison.

## À faire
- Migration `outbox` : file durable avec les états création, tentative, acceptation SMTP, copie dans Envoyés, résultat incertain ; identifiant d'opération unique ; Message-ID généré une fois.
- `POST /api/v1/drafts/{id}/send` avec identifiant d'opération : répond tout de suite avec un identifiant d'envoi ; `GET /api/v1/sends/{id}` donne l'état. Le même identifiant d'opération ne crée jamais deux envois.
- Validation avant mise en file : identité, adresses, disponibilité des pièces. Erreur bloquante distincte de l'objet vide, que l'utilisateur peut confirmer.
- Soumission SMTP authentifiée avec `lettre` 0.11 (vérifier la version) : STARTTLS sur le port du profil, identifiants du compte. Message MIME avec pièces et en-têtes de conversation.
- Après acceptation SMTP : copie dans Envoyés par `APPEND`, puis retrait du bon brouillon (local, serveur, IMAP). Un échec de copie relance uniquement la copie.
- La file reprend après redémarrage du serveur sans renvoyer un message déjà accepté.
- Front : états « en attente » puis « accepté par le serveur » ; bouton Envoyer insensible au double-clic.

## Critères d'acceptation
- [ ] Deux clics et une répétition HTTP donnent une seule soumission et un seul message reçu sur le banc.
- [ ] Un échec simulé de la copie dans Envoyés relance la copie seule ; une seule copie existe à la fin.
- [ ] Le brouillon envoyé disparaît de Brouillons, pas les autres.
- [ ] Un redémarrage entre acceptation SMTP et copie ne provoque pas de second envoi.
- [ ] La réponse reçue est rattachée au bon fil.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Retrouver un brouillon après une coupure », « Joindre, annuler et reprendre un fichier ».