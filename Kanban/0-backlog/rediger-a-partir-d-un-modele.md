---
id: k7qotjx6l
title: "Rédiger à partir d'un modèle"
depends_on: kd9c3y4qa, kk67sindc
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, front SolidJS dans `web/`, composeur, brouillons et identités existants). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte X06, ligne « Modèles et envoi différé ». Jalon Avancé : relis le code actuel du composeur avant de commencer.

## À faire
- Migration `templates` : par compte, nom, objet, corps (texte et HTML assaini).
- API `/api/v1/templates` : créer, lister, modifier, supprimer, pour le compte actif.
- Composeur : « Enregistrer comme modèle » et « Insérer un modèle ». L'insertion copie le contenu dans le brouillon ; le brouillon est ensuite librement modifiable et n'a plus de lien avec le modèle.
- Si le brouillon contient déjà du texte, proposer d'insérer à l'emplacement du curseur ou de remplacer.
- La liste des modèles est chargée à l'ouverture du composeur, sans retarder le premier affichage.
- Gestion des modèles dans les réglages du compte.

## Critères d'acceptation
- [ ] Enregistrer un modèle puis l'insérer dans deux brouillons.
- [ ] Modifier le premier brouillon ne change ni le modèle ni le second brouillon.
- [ ] Modifier le modèle ne change pas les brouillons déjà créés.
- [ ] Les modèles d'un compte n'apparaissent pas dans un autre.
- [ ] Un modèle contenant du HTML actif est assaini.

## Vérification
```
make check
```

## Dépendances
« Retrouver un brouillon après une coupure », « Personnaliser identité, signature et format de rédaction ».