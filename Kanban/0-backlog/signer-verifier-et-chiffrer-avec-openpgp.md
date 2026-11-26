---
id: kjfitaoa4
title: Signer, vérifier et chiffrer avec OpenPGP
depends_on: k1x8jlzjn, ktzs898x5, kdbgqguzb
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, composeur, pièces et envoi existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X08, ligne « Signature et chiffrement OpenPGP ». Jalon Avancé : relis le code actuel du composeur et du lecteur avant de commencer.

## À faire
- Intégrer OpenPGP.js (vérifier la version courante) dans un module chargé à la demande : il ne doit pas figurer dans le chargement initial.
- Clés : import d'une clé privée par identité, conservée chiffrée par sa phrase secrète dans le navigateur (jamais envoyée au serveur) ; carnet de clés publiques des correspondants par compte (import manuel, et recherche WKD si simple à ajouter).
- Composeur : options « Signer » et « Chiffrer ». Format PGP/MIME, pièces jointes comprises.
- Clé de destinataire manquante : le brouillon est conservé, l'adresse concernée est nommée, et l'envoi en clair exige une décision explicite de l'expéditeur.
- Lecteur : déchiffrement d'un message reçu ; trois états distincts et lisibles sans la couleur : signature valide, signature invalide, clé de vérification absente.
- Le contenu déchiffré passe par le même assainissement et le même isolement que tout autre message.
- Adapter le serveur pour accepter et transmettre tel quel un corps MIME déjà construit par le client.

## Critères d'acceptation
- [ ] Avec des clés d'essai, le destinataire déchiffre le texte et la pièce, puis vérifie la signature.
- [ ] Une signature altérée est signalée invalide.
- [ ] Avec une clé de destinataire manquante, rien ne part sans décision explicite.
- [ ] Le build montre OpenPGP dans un fichier séparé, absent du chargement initial.
- [ ] La clé privée n'apparaît dans aucune requête réseau (test).

## Vérification
```
make check
make test-integration
```

## Dépendances
« Répondre, répondre à tous et transférer », « Joindre, annuler et reprendre un fichier », « Soumettre une seule fois et classer dans Envoyés ».