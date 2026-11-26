---
id: kx7bmitqo
title: Ouvrir un message assaini et retrouver la liste
depends_on: kfi9t1244
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, module `web/src/sync/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte F08, lignes « Fil de lecture », « Lire et revenir », « Lire un contenu complexe », section « Frontières de confiance ». Le corps d'un message vient de n'importe quel expéditeur : il est assaini côté serveur et isolé côté navigateur.

## À faire
- `GET /api/v1/threads/{id}` (messages du fil) et `GET /api/v1/messages/{id}` (contenu). Le corps est récupéré à la demande par `BODY.PEEK[]`, analysé avec `mail-parser` 0.11.x, assaini avec `ammonia` (vérifier leurs versions courantes), puis conservé dans SQLite avec une version de rendu. La réponse porte un état de disponibilité.
- Assainissement : aucun script, formulaire ni gestionnaire d'événement ; ressources distantes bloquées par défaut et chargées sur demande pour ce message ; images intégrées servies par leur référence MIME via une route qui vérifie le compte ; citations et caractères internationaux préservés ; liste des pièces (nom, type, taille).
- Front : page de fil qui remplace seulement le contenu central. Objet et participants en tête, messages empilés, dernier déplié ; déplier un message antérieur conserve la position. HTML affiché dans une `iframe` avec `sandbox` sans scripts. Barre d'actions avec boutons provisoires (Répondre, Répondre plus tard, Mettre de côté, Plus).
- Lien direct `/fil/:thread/message/:id`.
- Ouverture explicite : marquer lu (indicateur `\Seen` posé en IMAP) ; le fil passe alors en « Déjà vus ».
- Retour : même section et même ligne, ou la voisine si le fil a changé de section, sans requête réseau bloquante.

## Critères d'acceptation
- [ ] Les corps texte et HTML du jeu `base` s'affichent ; un HTML avec `<script>`, `onerror` et image distante sort inoffensif et sans requête distante.
- [ ] Ouvrir A puis B : la réponse tardive de A laisse B à l'écran (test).
- [ ] Le retour à la liste ne déclenche aucun appel réseau bloquant (test avec `fetch` espionné).
- [ ] Le contenu d'un message d'un autre compte renvoie 404.
- [ ] Un préchargement (appel sans ouverture explicite) ne marque pas lu.

## Vérification
```
make check
make test-integration
```

## À vérifier par l'humain
- Rendu d'un fil déplié comparé à la planche ; retour à la liste ressenti comme instantané.

## Dépendances
« Accepter ou refuser depuis le Screener ».