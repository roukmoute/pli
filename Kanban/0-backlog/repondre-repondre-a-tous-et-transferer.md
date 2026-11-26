---
id: k1x8jlzjn
title: Répondre, répondre à tous et transférer
depends_on: kx7bmitqo, kklmrw0if
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, composeur et brouillons existants, fil de lecture existant). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C03, paragraphe « Réponse et transfert », ligne « Agir depuis un message ». L'envoi SMTP n'existe pas encore : cette carte produit des brouillons corrects.

## À faire
- Back : `POST /api/v1/messages/{id}/reply`, `reply-all`, `forward` créent un brouillon prérempli.
- Répondre : destinataire pris dans Reply-To, sinon From. Répondre à tous : À et Cc reconstruits, propres adresses du compte exclues, doublons retirés. Les Cci d'un envoi précédent ne sont jamais repris.
- En-têtes In-Reply-To et References posés pour conserver le lien de conversation ; préfixe d'objet non répété.
- Citation du message choisi, à partir du contenu assaini : aucun contenu actif n'est réintroduit.
- Transfert : contenu et pièces jointes applicables repris en référence.
- Front : Répondre, Répondre à tous et Transférer ciblent le message actif du fil. Réponse rapide sous le message, dans le fil. À la fermeture, retour au fil d'origine.

## Critères d'acceptation
- [ ] Une réponse à un message portant Reply-To s'adresse à cette adresse.
- [ ] Répondre à tous sur un message reçu avec Cc exclut l'adresse du compte et n'a aucun doublon.
- [ ] Le brouillon de réponse porte In-Reply-To et References corrects.
- [ ] Citer un message HTML piégé ne réintroduit ni script ni image distante.
- [ ] Fermer la réponse rouvre le fil d'origine sur le bon message.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Ouvrir un message assaini et retrouver la liste », « Composer un premier brouillon ».