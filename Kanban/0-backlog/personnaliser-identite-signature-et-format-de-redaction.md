---
id: kk67sindc
title: Personnaliser identité, signature et format de rédaction
depends_on: k1x8jlzjn, kdbgqguzb
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, composeur, réponses et envoi existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C09, ligne « Identités et préférences ». Jalon Parité : relis le code actuel du composeur avant de commencer.

## À faire
- Migration `identities` : par compte, nom affiché, adresse, signature (texte et HTML), identité par défaut. Une identité créée automatiquement au rattachement de la boîte.
- Migration ou table de préférences de rédaction par compte : format par défaut (texte ou HTML), raccourci d'envoi Ctrl ou Cmd + Entrée activé ou non.
- API `/api/v1/identities` et préférences de rédaction.
- Écran de réglages depuis le menu de l'avatar : nom affiché, signature, format, raccourci d'envoi.
- Composeur : une nouvelle rédaction prend l'identité du compte actif et applique sa signature ; un brouillon existant conserve l'identité déjà choisie. Sélecteur d'identité visible quand plusieurs existent.
- Répondre à tous exclut toutes les identités du compte.
- L'en-tête From envoyé correspond à l'identité choisie ; la signature HTML est assainie.

## Critères d'acceptation
- [ ] Changer le nom affiché et la signature se retrouve dans un nouveau message, pas dans un brouillon existant.
- [ ] Le mode texte envoie un message sans partie HTML.
- [ ] Ctrl ou Cmd + Entrée n'envoie que si le réglage est activé.
- [ ] Le message reçu sur le banc porte le bon From et la signature.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Répondre, répondre à tous et transférer », « Soumettre une seule fois et classer dans Envoyés ».