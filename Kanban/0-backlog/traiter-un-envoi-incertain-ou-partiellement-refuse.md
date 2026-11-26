---
id: kt6ats33l
title: Traiter un envoi incertain ou partiellement refusé
depends_on: kdbgqguzb
---

## Contexte
Webmail Infomaniak à interface HEY (back Rust dans `server/`, file d'envoi `outbox`, front SolidJS dans `web/`). Cahier : `Kanban/.specs/piece-TlW9Y4/cahier-des-charges-webmail-infomaniak-hey.md`, carte C08, paragraphe « Envoi fiable » (et la RFC 5321, section 4.5.3.2.6, citée par le cahier). Si la connexion tombe après la remise des données, on ne sait pas si le serveur a accepté le message.

## À faire
- Écrire un faux serveur SMTP scriptable pour les tests Rust (refus d'un destinataire, coupure après `DATA`, erreur temporaire, erreur définitive) : le banc GreenMail ne sait pas injecter ces pannes.
- État « envoi à vérifier » quand l'acceptation est incertaine : aucun renvoi automatique.
- Rapprochement : chercher le Message-ID dans le dossier Envoyés et dans les traces applicatives ; proposer ensuite à l'utilisateur de confirmer l'envoi ou d'envoyer de nouveau.
- Refus par destinataire : conserver la réponse SMTP de chaque `RCPT TO` ; un destinataire refusé parmi plusieurs reste identifiable, les acceptés aussi.
- Erreurs SMTP classées (identification, destinataire, taille, temporaire, définitive), sans adresse complète dans les journaux.
- Les erreurs temporaires sont retentées avec délai croissant ; les définitives rendent le brouillon à l'utilisateur.
- Front : affichage de ces états dans le composeur et dans une liste « Envois » accessible depuis le menu.

## Critères d'acceptation
- [ ] Une coupure après la remise des données donne « envoi à vérifier » et aucun second envoi automatique.
- [ ] Si le message est retrouvé dans Envoyés, l'état passe à accepté sans action.
- [ ] Avec trois destinataires dont un refusé, l'écran nomme le refusé et les deux acceptés.
- [ ] Un refus SMTP définitif rend le brouillon intact.

## Vérification
```
make check
make test-integration
```

## Dépendances
« Soumettre une seule fois et classer dans Envoyés ».