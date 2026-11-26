---
id: kp6ohvyka
title: "Tester la rédaction et l'envoi de bout en bout"
depends_on: k7pfstynd, kq9cyz1p6, kt6ats33l
---

## Contexte
Webmail Infomaniak à interface HEY (tests Playwright dans `e2e/`, `make e2e`, banc IMAP/SMTP dans `bench/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, paragraphe « Recette intégrée » de la section « Rédaction et envoi ». Cette carte automatise cette recette.

## À faire
- Scénario nominal : répondre à tous à un message reçu avec Cc, ajouter une pièce, attendre la sauvegarde locale, fermer le navigateur, le rouvrir, reprendre le brouillon ; contrôler identité, destinataires et pièce ; envoyer.
- Vérifier côté banc : réception par le destinataire de test, rattachement au fil (en-têtes), contenu de la pièce identique, une seule copie dans Envoyés, retrait du bon brouillon.
- Scénario double-clic sur Envoyer : un seul message reçu.
- Scénario refus SMTP : rejouer avec un destinataire refusé ; le brouillon est préservé et le refus identifié. Utiliser le faux serveur SMTP scriptable des tests Rust, exposé au serveur du webmail par un profil de test.
- Scénario succès SMTP suivi d'un échec de copie dans Envoyés : la copie est relancée seule.
- Scénario passage de la réponse rapide au composeur complet, minimisation et reprise.
- Corriger les défauts mineurs trouvés ; lister les autres en fin de journal de carte.

## Critères d'acceptation
- [ ] Les six scénarios passent dans `make e2e`, sans écran.
- [ ] Aucun scénario ne produit de double envoi ni de brouillon perdu.
- [ ] Les vérifications lisent l'état réel IMAP/SMTP du banc.
- [ ] Le job e2e de la CI les inclut.

## Vérification
```
make e2e
make check
```

## Dépendances
« Tester le jalon Lecture de bout en bout avec Playwright », « Passer de la réponse rapide à la rédaction complète », « Traiter un envoi incertain ou partiellement refusé ».