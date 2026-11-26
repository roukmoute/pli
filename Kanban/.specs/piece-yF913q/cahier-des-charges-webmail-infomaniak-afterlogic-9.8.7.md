# Cahier des charges du webmail Infomaniak avec interface Afterlogic 9.8.7

Le futur webmail permettra de traiter une boîte Infomaniak avec les gestes d’Afterlogic WebMail Pro 9.8.7. Sur ordinateur, l’utilisateur passe des dossiers à la liste puis au message, répond et classe son courrier sans perdre sa place. Sur téléphone, il retrouve le même dossier et le même message dans des écrans successifs. La première livraison vise une installation personnelle sur un serveur, avec la possibilité d’y rattacher plusieurs boîtes.

Le résultat attendu se juge sur un geste simple. L’utilisateur lit le troisième message d’une liste, l’archive, voit le suivant, quitte Réception puis y revient. Le dossier, la position et les compteurs sont cohérents. La liste déjà consultée réapparaît en moins de 50 ms dans 95 % des essais selon le protocole de recette ci-dessous. Cette durée est une cible du nouveau produit.

## Référence produit et priorités

| Référence | Ce qu’elle établit pour le projet |
| --- | --- |
| Installation personnelle, consultée le 25 novembre 2026 | L’administration affiche **9.8.7.build7-build-o3**. Le thème est **Default**, la langue est autodétectée et l’heure est au format 24 h. L’affichage du courrier est vertical et le changement de disposition par l’utilisateur est désactivé. L’ajout de boîtes externes et la création du profil à la première connexion valide sont autorisés. [Installation](https://webmail.strasser.yt/) |
| Livraison publique correspondante | Le dépôt nommé *webmail-pro-8* contient aussi les versions 9. Le commit public **3e32764** correspond au build 7 de 9.8.7. Son manifeste verrouille notamment le module courrier en **0.10.143**. Les modèles d’écran et les gestionnaires d’événements de cette livraison précisent les interactions. [Version](https://github.com/afterlogic/webmail-pro-8/blob/3e327646fdb7f7107439aa85d75450b74438d18c/VERSION) · [Modules du build](https://github.com/afterlogic/webmail-pro-8/blob/3e327646fdb7f7107439aa85d75450b74438d18c/composer.lock) |
| Écrans accessibles | La connexion et le panneau administrateur ont été parcourus sur l’instance personnelle. Le panneau indiquait zéro utilisateur et une liste de serveurs vide. La démo publique a présenté le formulaire prérempli, puis le chargement de la boîte a échoué. Les parcours de courrier ci-dessous s’appuient donc sur le code du build et les vues publiées par l’éditeur. La recette sur boîte de test validera les détails dépendants du compte. [Démo](https://webmail.afterlogic.com/) · [Vue officielle de la messagerie](https://afterlogic.com/images/products/wmp8/screens/message-list.png) |
| Décisions propres au futur webmail | Rust, TypeScript, affichage depuis le cache, archivage optimiste, recherche locale et budgets de latence répondent à l’objectif de rapidité. Pour un écart d’ergonomie, l’installation personnelle 9.8.7 fait foi. La charte et les ressources graphiques réutilisées ont une provenance consignée. Le dépôt précise la licence du produit et celles de ses modules. [Notice du projet](https://github.com/afterlogic/webmail-pro-8/blob/3e327646fdb7f7107439aa85d75450b74438d18c/README.md#licensing) |

**Ordre de livraison proposé.** Le premier jalon fait lire un véritable message Infomaniak dans les trois volets. Le deuxième permet de travailler au quotidien sur ordinateur et mobile, avec rédaction, brouillons, pièces jointes, classement et recherche. Le troisième couvre le confort de parité, les préférences, le multicomptes et l’exploitation. Les extensions de collaboration ont chacune une recette et une décision d’activation. Le gain de vitesse se mesure dès le premier jalon.

## Écrans et fidélité visuelle

**Connexion.** L’écran observé présente un fond abstrait pleine fenêtre, corail et pêche en haut, turquoise au centre, bleu pétrole dans les diagonales du bas. Un symbole de messagerie surmonte deux champs blancs empilés dans un formulaire compact centré. Le champ actif est reconnaissable. Le bouton turquoise « Se connecter » occupe la largeur des champs. Le choix de langue se trouve en haut à droite et la signature du produit en bas. Dans la nouvelle interface, Entrée soumet une seule fois. Une erreur laisse l’identifiant modifiable, place le focus utile et distingue un échec d’identification d’un service temporairement indisponible. [Écran de référence](https://webmail.strasser.yt/)

**Bureau.** Une navigation horizontale donne accès au courrier et aux modules activés. Sous cette navigation, les séparateurs verticaux donnent environ **20 % aux dossiers, 30 % à la liste et 50 % à la lecture** à l’ouverture. Le réglage de largeur se conserve pour l’appareil. Un séparateur peut se manipuler au pointeur et au clavier. Dossiers, liste et corps du message possèdent leurs propres zones de défilement. Le mode vertical est le réglage de l’instance de référence. [Gabarit du build](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/templates/MailVerticalLayoutView.html) · [Vues de l’éditeur](https://afterlogic.com/docs/webmail-pro/configuring-webmail/how-to-enable-horizontal-layout)

| Zone | Composition et comportements attendus |
| --- | --- |
| Volet gauche | « Nouveau message » est la première action. Viennent le compte actif, les dossiers système, la vue des messages marqués et l’arborescence personnelle selon leur disponibilité. Le dossier actif possède un fond distinct. Un compteur indique les non-lus. Un chevron replie un dossier parent sans changer le dossier consulté. |
| Volet central | Barre d’actions de liste, sélection globale de la page, recherche et lignes compactes. Chaque ligne donne l’expéditeur ou le destinataire suivant le dossier, l’objet et la date. La case, l’étoile, la pièce jointe, l’état de réponse ou de transfert et le nombre de messages du fil ont des fonctions distinctes. Un objet long se tronque dans la liste et reste intégralement accessible à l’ouverture. |
| Volet droit | En haut, expéditeur et destinataires sont condensés. L’objet est plus large, la date reste visible et les détails se déplient. Répondre, répondre à tous, transférer, ouvrir séparément et les actions supplémentaires restent à portée. Le corps, les pièces jointes et la réponse rapide occupent ensuite le panneau. |
| Apparence et états | Le thème Default fournit des repères précis. Action principale turquoise **#43D0BF**, ligne ouverte bleue **#78B8F0**, ligne cochée vert pâle **#EBF7CB**, non-lu gris clair **#F4F6F7**, icônes de barre **#82748E**. Le non-lu utilise aussi la graisse, la sélection multiple une case visible et le focus un contour. Les barres blanches et les séparations fines maintiennent la densité observée. [Thème du build](https://github.com/afterlogic/aurora-module-webclient-core/blob/0.10.135/styles/themes/Default/styles.less) |

**Navigation et écran étroit.** Un lien profond ouvre le bon compte, dossier et message sous réserve d’autorisation. Précédent et Suivant du navigateur restaurent filtre, tri et position. Sur téléphone, l’arborescence devient un tiroir, ouvrir une ligne donne la lecture en plein écran et Retour restitue la liste au même endroit. Le clavier virtuel laisse l’action Envoyer et les destinataires accessibles. Sur tablette, la disposition dépend de la largeur utile. Les essais de conception couvrent 390 × 844, 768 × 1024 et 1440 × 900 pixels CSS, le zoom à 200 %, le tactile et la navigation au clavier. [Écrans mobiles du build](https://github.com/afterlogic/aurora-module-mobile-webclient-mail/tree/0.10.18/vue-mobile/pages)

**Recette visuelle.** Avec une même boîte fictive, comparer la connexion, la liste vide, une ligne non lue, une ligne ouverte avec deux autres cochées, un fil déplié, un HTML long, la recherche avancée et un brouillon avec pièce jointe. Pour chaque vue, vérifier l’ordre des actions, l’alignement, la largeur des volets, le contraste, le focus après fermeture et la zone qui défile. Le contrôle porte autant sur la séquence des gestes que sur la capture finale.

## Lecture et traitement du courrier

**Première connexion et retour.** Après authentification IMAP réussie, le backend découvre les dossiers et leurs rôles. Il charge d’abord les en-têtes de la première page de Réception, puis complète l’historique en arrière-plan. La préférence publique du build est de 20 messages par page. Le nouveau client peut récupérer un lot plus grand en arrière-plan tout en conservant la pagination choisie à l’écran. Au retour, une session active sur un appareil de confiance retrouve la dernière liste depuis la mémoire ou IndexedDB, avec la date de dernière synchronisation si le réseau est indisponible. [Préférences du module courrier](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/Settings.php)

| Geste | Résultat à reproduire et cas de recette |
| --- | --- |
| Cliquer sur une ligne | Ouvre l’aperçu à droite. Le passage à lu suit la préférence, activée par défaut dans le module. La ligne, le compteur du dossier et le titre éventuel se mettent à jour une seule fois. Un préchargement du corps conserve l’état non lu. |
| Cocher, puis agir | Cocher B et C en gardant A ouvert doit appliquer une action de liste à **B et C**. Quand la sélection cochée est vide, l’action s’applique au message ouvert si l’action est permise. La case d’en-tête représente la page affichée et possède un état intermédiaire. La destination et le nombre de messages sont visibles avant une action de lot. |
| Double-cliquer ou déplier | Un double-clic sur un brouillon ouvre la rédaction, un double-clic sur un courrier ouvre sa vue dédiée. Le bouton du fil déplie les messages liés. Entrée sur le fil le développe ou charge ses éléments suivants. Le clic sur une étoile ou une case garde la lecture en cours. [Gestionnaire de liste](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/js/views/CMessageListView.js) |
| Trier, paginer ou remonter | Conserver le sens du tri, le filtre et l’ancre de défilement par dossier. À date identique, utiliser un second identifiant pour stabiliser l’ordre. Si un nouveau message arrive pendant la lecture d’une page ancienne, la ligne sous le pointeur garde sa place. Une notification permet de revenir aux nouveautés. |
| Déplacer ou copier | Le menu de dossier et le glisser-déposer proposent la même destination. Dans la référence, Ctrl ou Cmd pendant le glisser demande une copie. Une opération optimiste retire ou ajoute les lignes pertinentes, corrige les compteurs et conserve la sélection utile. Un refus restitue les éléments concernés et explicite un éventuel succès partiel. [Gestes de classement](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/js/views/CMailView.js) |
| Archiver, signaler ou supprimer | L’archivage demandé déplace vers le dossier d’archives associé au compte. À défaut d’association, proposer le choix du dossier. Le spam, sa sortie, la mise à la corbeille et la restauration suivent leurs propres destinations. La purge définitive affiche le dossier, le nombre d’éléments et une confirmation dédiée. |
| Ouvrir un contenu complexe | Le texte, le HTML assaini, les citations, les caractères internationaux et les contenus intégrés restent lisibles. Les images distantes sont bloquées par défaut avec une action explicite pour les afficher. Une pièce jointe présente nom, type, taille et téléchargement à la demande. L’aperçu d’un fichier compatible utilise un contexte isolé. |
| Agir depuis la lecture | Différencier Répondre, Répondre à tous et Transférer. Le menu secondaire du build comprend notamment imprimer, télécharger en EML, transférer en pièce jointe et consulter les en-têtes. Les boutons indisponibles ont un état identifiable. [Commandes du lecteur](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/templates/Message/ToolbarView.html) |

**Dossiers et conversations.** Réception, Envoyés, Brouillons, Indésirables, Corbeille et Archives sont associés à leurs rôles IMAP, y compris avec des libellés étrangers. Les favoris constituent une vue filtrée. Créer, renommer, déplacer, masquer ou réafficher un dossier se reflète dans les choix de classement. Une conversation regroupe les réponses liées quand l’information le permet. Deux messages indépendants ayant le même objet restent distincts. L’utilisateur peut retrouver le message précis d’un fil depuis une recherche.

**Recherche.** Le champ compact accepte une recherche simple. Le panneau avancé propose expéditeur, destinataire, objet, texte, bornes de dates, présence de pièce jointe et, selon la capacité, son nom. Le périmètre se choisit entre dossier courant, dossier et sous-dossiers ou tous les dossiers du compte. Effacer la recherche restitue la vue précédente. Une requête plus ancienne qui se termine en retard ne peut écraser les résultats courants. Un index incomplet signale sa couverture et propose de poursuivre en ligne. « Aucun résultat » correspond à une recherche terminée dans le périmètre affiché. [Formulaire du build](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/templates/SearchView.html)

**Clavier.** En dehors d’un champ de saisie, S place le curseur dans la recherche, N compose, R répond et Q cible la réponse rapide lorsqu’elle est disponible. Les flèches parcourent la liste. Dans la rédaction, Ctrl+S sauvegarde le brouillon. Ctrl+Entrée envoie uniquement si l’option est activée et si les conditions d’envoi sont remplies. Taper « réunion » dans le corps d’un message doit simplement saisir le mot. Après fermeture d’un menu, le focus revient à sa commande. [Raccourcis de liste](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/js/views/CMailView.js) · [Raccourcis de lecture](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/js/views/MessagePaneView.js) · [Rédaction](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/js/views/CComposeView.js)

## Rédaction et envoi

**Nouveau message.** Le bouton permanent ouvre le composeur. Le compte et l’identité d’envoi sont visibles quand plusieurs choix existent. Le focus initial va au destinataire. À, Cc et Cci acceptent des adresses collées ou suggérées, des noms accentués et plusieurs destinataires. Chaque adresse invalide peut être corrigée individuellement. Un même destinataire ajouté deux fois est signalé. L’utilisateur saisit l’objet, compose en texte ou HTML, applique sa signature et ajoute ses fichiers. Le corps conserve listes, liens et citations après fermeture puis réouverture. [Écran de composition](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/templates/ComposeView.html)

**Réponse et transfert.** Répondre respecte l’adresse de réponse du message. Répondre à tous reconstruit À et Cc, tient compte des propres identités du compte et évite les doublons. Les destinataires Cci d’un envoi restent privés. Une réponse conserve le lien de conversation et cite le message choisi. Un transfert reprend le contenu et les pièces jointes applicables. Passer de la réponse rapide au composeur complet conserve le texte saisi, l’identité et les destinataires. Le retour au dossier rétablit le message d’origine.

**Brouillons.** Un identifiant stable représente le brouillon depuis la première saisie. L’état distingue « enregistré sur cet appareil », « synchronisation en cours » et « enregistré dans Brouillons ». Fermer l’éditeur propose de conserver ou d’abandonner les modifications si nécessaire. Une pièce en cours de transfert affiche sa progression et son erreur éventuelle. Après coupure, l’utilisateur retrouve son texte et distingue les fichiers qui doivent être joints à nouveau. Deux onglets qui modifient le même brouillon doivent conserver les deux versions en cas de conflit. La minimisation du composeur laisse l’utilisateur consulter un autre message. [Comportement du composeur](https://github.com/afterlogic/aurora-module-webclient-mail/blob/0.10.143/js/views/CComposeView.js)

**Envoi fiable.** Au clic sur Envoyer, valider l’identité, les adresses et la disponibilité des pièces. Distinguer l’erreur bloquante d’un objet vide que l’utilisateur peut confirmer. Un identifiant d’opération neutralise le double-clic et les répétitions HTTP. Afficher « en attente » tant que l’envoi est local, puis « accepté par le serveur » après la réponse SMTP positive. Un échec de copie dans Envoyés relance uniquement cette copie. Si la connexion tombe au moment où l’acceptation SMTP devient incertaine, présenter l’état « envoi à vérifier », rapprocher avec les traces applicatives et le dossier Envoyés, puis laisser décider d’un nouvel envoi. Un destinataire refusé parmi plusieurs reste identifiable. [Sémantique SMTP](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.6)

**Recette intégrée.** Sur une boîte de test, répondre à tous à un message reçu avec Cc, ajouter une pièce, fermer le navigateur après sauvegarde locale et reprendre le brouillon. Contrôler l’identité, les destinataires et la pièce avant envoi. Vérifier la réception sur une adresse de test consentie, le rattachement au fil, une seule copie dans Envoyés et le retrait du bon brouillon. Rejouer avec refus SMTP et avec succès SMTP suivi d’un échec de classement.

## Préférences et modules autour du courrier

| Domaine | Parcours à couvrir | Décision de livraison |
| --- | --- | --- |
| Identités et préférences | Changer le nom affiché, la signature, le format de rédaction, le nombre de lignes par page, la langue et le fuseau. Une nouvelle rédaction reprend l’identité du compte actif. Un brouillon existant conserve celle déjà choisie. | Préférences de lecture et d’envoi dans le courrier quotidien. Identités multiples et réglages avancés ensuite. |
| Comptes liés et réception unifiée | Ajouter une seconde boîte, changer de compte, reconnaître l’origine de chaque ligne dans la vue unifiée et répondre avec le bon expéditeur. Une suppression de profil explique ses effets locaux. Une boîte distante continue à vivre chez son hébergeur. | Un compte Infomaniak à la première livraison. Multicomptes ensuite, conformément à l’option autorisée sur l’instance. |
| Contacts | Rechercher, ouvrir, créer depuis un expéditeur, modifier et utiliser une fiche dans l’autocomplétion. Gérer plusieurs adresses et groupes. Prévisualiser imports CSV ou vCard, doublons et lignes en erreur. Exporter une sélection. | Suggestions et carnet personnel d’abord. Groupes, import/export et synchronisation CardDAV en lots distincts. [Module Contacts](https://github.com/afterlogic/aurora-module-webclient-contacts/tree/0.10.57/templates) |
| Calendrier et invitations | Accéder aux vues jour, semaine et mois. Créer un événement avec fuseau, gérer les récurrences, puis modifier une occurrence ou toute la série. Depuis un mail d’invitation, afficher l’organisateur et répondre avec le bon compte. | Extension de parité. Le panneau personnel observé propose la vue mois, un début de semaine dimanche et une plage de travail de 9 h à 18 h. Le fuseau personnel commande le rendu des heures. [Module Calendrier](https://github.com/afterlogic/aurora-module-webclient-calendar/tree/0.10.91/templates) |
| Fichiers et pièces jointes | Depuis un mail, télécharger ou enregistrer un fichier. Depuis le composeur, joindre un fichier disponible. Dans l’espace Fichiers, parcourir, renommer, déplacer, télécharger et gérer les limites de taille et d’espace. Pour un lien partagé, préciser le public, l’échéance et la révocation. | Pièces jointes essentielles dès la rédaction. Espace de fichiers et partage après activation d’un stockage et de ses permissions. [Module Fichiers](https://github.com/afterlogic/aurora-module-webclient-files/tree/0.10.74/templates) |
| Tâches et notes | Une tâche possède un titre, une échéance et un état. Une note peut être créée, retrouvée, éditée et supprimée dans le périmètre du compte. La navigation revient au courrier dans son état précédent. | Extensions indépendantes, activées selon le besoin. La distribution référence des modules distincts. [Manifeste du build](https://github.com/afterlogic/webmail-pro-8/blob/3e327646fdb7f7107439aa85d75450b74438d18c/composer.json) |
| Règles, absence et transfert automatique | Présenter les règles, leur ordre et leur état, puis vérifier une modification sur une boîte de recette. La disponibilité des filtres côté hébergeur dépend d’une interface prise en charge. | Décision après vérification du service Infomaniak correspondant. CalDAV, CardDAV, ManageSieve et IMAP représentent des capacités distinctes. [Configuration Afterlogic](https://afterlogic.com/docs/webmail-pro/configuring-webmail/using-managesieve-with-webmail) |
| Autres options de la distribution | Envoi programmé et annulation avant échéance, modèles, chiffrement OpenPGP, services externes et connexion fédérée ont des données, autorisations et scénarios de récupération propres. | Une décision et une carte d’exploration par option retenue. Le premier démarrage du courrier charge uniquement les modules nécessaires à son affichage. |

## Administration et exploitation

**Accès administrateur distinct.** Les rubriques observées comprennent Système, Utilisateurs et Déconnexion. Système regroupe notamment compte administrateur, base, journalisation, réglages communs, marque, courrier, serveurs de messagerie, calendrier, fichiers et synchronisation mobile. La présence d’une rubrique de service externe indique une possibilité de configuration. Pour le produit Rust, un administrateur gère les profils de connexion, les politiques et la santé du service. Les opérations sur le contenu d’une boîte relèvent de la session autorisée de cette boîte. [Panneau de référence](https://webmail.strasser.yt/adminpanel/)

| Écran cible | Action et résultat vérifiables |
| --- | --- |
| Premier démarrage | Si aucun serveur n’est défini, afficher l’état vide et « Ajouter un serveur ». Le préréglage Infomaniak expose le nom d’hôte, IMAP/TLS et SMTP/STARTTLS. Le test de connexion rend des résultats séparés pour la résolution, TLS et l’authentification, sans envoyer de courrier à un tiers. |
| Serveurs et utilisateurs | Associer domaines et profils, décider de l’auto-provisionnement, autoriser les boîtes externes et afficher le nombre de profils actifs. Avant de désactiver un profil, montrer les comptes concernés. Une configuration invalide conserve la dernière configuration fonctionnelle. |
| Apparence et modules | Appliquer thème, langue, format horaire, disposition et limites. Indiquer le réglage global, la surcharge personnelle permise et l’effet sur une prochaine session. Les intégrations désactivées disparaissent des menus, du chargement initial et des suggestions d’action. |
| Exploitation | Afficher la dernière synchronisation réussie, le nombre d’actions en attente, le retard d’indexation, les erreurs SMTP classées et l’espace utilisé. Permettre une resynchronisation ciblée ou la reconstruction d’un index tout en donnant accès aux données déjà disponibles. |
| Sauvegarde et reprise | Sauvegarder préférences, contacts locaux, brouillons non transmis, opérations en attente et références aux fichiers. Restaurer sur une instance de test, rapprocher les opérations déjà acceptées et reconstruire les caches. Vérifier que la reprise ne réexpédie pas le courrier. |

## Architecture et cohérence des données

**Architecture de départ.** Le navigateur TypeScript dialogue en HTTPS avec un service Rust utilisant Axum et Tokio. Un processus de synchronisation, qui peut vivre dans le même binaire, communique avec Infomaniak. L’API sert les lectures courantes depuis SQLite sur SSD. La projection locale contient dossiers, en-têtes, indicateurs, résumés et état de synchronisation. Les corps récents et les fichiers ont un budget de stockage distinct. L’hébergeur reste la référence pour les messages et dossiers déjà synchronisés. Les brouillons en attente et la file d’envoi exigent une conservation durable.

**Infomaniak.** Accès entrant sur `mail.infomaniak.com:993` en TLS implicite, soumission authentifiée sur `mail.infomaniak.com:587` avec STARTTLS, identifiant égal à l’adresse complète et secret d’appareil côté serveur. Le serveur public annonçait IMAP IDLE lors de la vérification préalable. Détecter de nouveau les capacités une fois authentifié. IDLE surveille le dossier sélectionné. Une connexion de surveillance est mutualisée pour les onglets d’un même compte. Les autres dossiers sont vérifiés à l’ouverture et périodiquement, dans les limites du fournisseur. [Paramètres Infomaniak](https://www.infomaniak.com/fr/support/faq/2427/synchroniser-les-e-mails-sur-tous-vos-equipements) · [IMAP IDLE](https://www.rfc-editor.org/rfc/rfc9051.html#section-6.3.13)

| Donnée | Identité, propriétaire et règle de reprise |
| --- | --- |
| Compte et dossier | Un compte applicatif est associé à son principal authentifié. Un dossier possède un identifiant interne, un chemin distant et un rôle. Vérifier les droits sur chaque requête HTTP, flux d’événements et fichier. |
| Message et indicateurs | Pour IMAP, un UID identifie un message dans un dossier et une génération UIDVALIDITY. La clé comporte compte, dossier, UIDVALIDITY et UID. Le Message-ID sert au rapprochement et ne constitue pas une clé unique. Une copie ou un déplacement peut attribuer un nouvel UID. |
| Résumés et recherche | Indexer par compte, dossier, date de réception et identifiant stable. Paginer par curseur. Utiliser FTS5 pour le texte indexé, segmenter par compte et supprimer les entrées lors d’un retrait. Un corps arrivé tardivement enrichit l’index sans créer une seconde ligne de courrier. [SQLite FTS5](https://www.sqlite.org/fts5.html) |
| Brouillon et envoi | Conserver identifiant local, version, compte émetteur, destinataires, contenu et références de pièces. La file durable distingue création, tentative, acceptation SMTP, copie dans Envoyés et résultat incertain. |
| Changements et actions locales | Chaque action possède un identifiant stable, une portée, son état attendu et son résultat. La mise à jour de la projection et la nouvelle révision sont enregistrées dans une seule transaction. Deux attentes opposées, par exemple lu puis non lu, convergent vers la dernière intention applicable. |

**Synchroniser vite et juste.** Récupérer les en-têtes par lots et les corps avec une lecture qui préserve le drapeau lu. À la reconnexion, utiliser QRESYNC ou CONDSTORE si disponibles et, sinon, rapprocher UID et indicateurs par lots, suppressions comprises. Un changement d’UIDVALIDITY invalide la correspondance distante du dossier. Reconstruire celui-ci en priorité et mettre les opérations portant sur d’anciens identifiants en attente de vérification. Tester l’arrivée d’un message pendant un déplacement, la suppression depuis un téléphone et la restauration après une veille prolongée. [Extensions de resynchronisation IMAP](https://www.rfc-editor.org/rfc/rfc7162.html)

**Contrat entre front et back.** Une lecture de dossier renvoie le compte, le dossier, les résumés de la page, le curseur suivant, une révision et la couverture de synchronisation. La lecture d’un message renvoie le contenu assaini avec son état de disponibilité. Une commande de classement transmet les identifiants ciblés et retourne un résultat par élément. Une commande d’envoi retourne un identifiant consultable. Le flux SSE annonce des révisions privées. À la reconnexion, le client demande les changements après sa dernière révision appliquée. Si elle a expiré, il reprend un instantané du dossier. Un événement reçu deux fois produit le même état. [SSE avec Axum](https://docs.rs/axum/latest/axum/response/sse/)

## Caches et réactivité

| Emplacement | Contenu et moment de lecture | Fraîcheur et éviction |
| --- | --- | --- |
| Mémoire de l’onglet | Dossier visible, lignes à l’écran, petite fenêtre de lignes voisines et message courant. Revenir depuis un autre dossier dessine d’abord les données disponibles. | Appliquer les changements par identifiant. Une réponse tardive peut alimenter le cache du bon dossier, mais seul le dossier encore actif reçoit le rendu. |
| IndexedDB sur appareil de confiance | Listes récentes, derniers corps assainis, préférences de présentation, brouillons locaux et révision appliquée. Ouvrir d’abord l’interface, puis relire la liste sans appel IMAP bloquant. | Isoler par utilisateur et compte. Limiter le volume et évincer les lectures anciennes avant les brouillons et opérations en attente. Informer si le stockage est plein. Réinitialiser à la déconnexion selon la politique explicite de l’appareil. [IndexedDB](https://developer.mozilla.org/fr/docs/Web/API/IndexedDB_API) |
| Cache HTTP | JS, CSS, polices et icônes identifiés par une empreinte, avec cache long. Charger le composeur riche, le calendrier et le lecteur de pièces à leur première utilisation. | Une nouvelle version change l’empreinte. Le cache partagé sert les ressources publiques. L’API et les données personnelles suivent une politique privée. Sur un appareil partagé, limiter les traces persistantes et proposer d’enregistrer un brouillon en ligne avant la déconnexion. [Cache HTTP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Caching) |
| SQLite et fichiers applicatifs | Stockage indexé des résumés, compteurs et corps récents. Le mode WAL permet aux lectures de continuer pendant les écritures du synchroniseur. Les fichiers volumineux se lisent en flux. | Versionner le rendu HTML assaini, gérer les suppressions et quotas, interrompre les travaux devenus inutiles. Les transactions de base sont courtes. Surveiller la taille du WAL, les blocages et le retard d’indexation. [SQLite WAL](https://www.sqlite.org/wal.html) |
| Redis et Mercure | Sur une instance, le flux SSE privé d’Axum relie directement le service au navigateur. Évaluer Redis pour des lectures coûteuses relues par plusieurs instances et Mercure pour externaliser les connexions temps réel et leurs abonnements privés. | Décider sur un essai avec et sans le composant, au même débit et au même p95. Clés par compte et version. La révision durable permet le rattrapage après un événement manqué. Redis Pub/Sub distribue ses événements au plus une fois. [Mercure](https://mercure.rocks/spec) · [Redis Pub/Sub](https://redis.io/docs/latest/develop/pubsub/#delivery-semantics) |

**Priorité réseau.** La lecture demandée par l’utilisateur passe avant le préchargement, qui passe avant l’indexation de l’historique. Précharger au repos un petit corps probable, par exemple le message voisin d’une lecture au clavier. Annuler sur changement rapide de dossier, économiseur de données ou réseau contraint. Grouper les requêtes identiques déjà en cours. Télécharger les grosses pièces à la demande, avec progression et annulation. Conserver l’ancre de défilement lors d’un ajout ou d’un changement de hauteur.

**Reprise après incident.** Hors ligne, une lecture en cache indique sa fraîcheur. Un message jamais ouvert affiche un état « contenu disponible à la reconnexion ». Une action mise en attente porte un indicateur distinct de l’action confirmée. À la réouverture d’un onglet, comparer sa révision à celle du serveur avant de rejouer les commandes. Une action devenue impossible, par exemple un déplacement vers un dossier supprimé, est expliquée et résolue par l’utilisateur. La purge d’un compte retire ses caches, ses index et ses fichiers temporaires selon la politique de conservation choisie.

## Mesure des performances et recette

Les objectifs portent sur le contenu effectivement visible, la cohérence des données et la latence de l’action confirmée. Un squelette de chargement ne termine aucune mesure d’affichage de contenu. La vitesse du fournisseur, le temps CPU du navigateur, le temps serveur et les attentes réseau sont enregistrés séparément.

| Parcours chronométré | Conditions de référence | Cible initiale au 95ᵉ percentile |
| --- | --- | --- |
| Clic sur Réception → première liste lisible | Onglet actif, liste en mémoire | Moins de 50 ms et zéro requête réseau bloquante |
| Clic → texte d’un petit message récent visible | Corps déjà dans IndexedDB, ressources distantes facultatives | Moins de 100 ms |
| Archiver → ligne et compteur visiblement mis à jour | Boîte chargée, un élément sélectionné | Moins de 50 ms. Chronométrer séparément la confirmation distante |
| Ouverture → première page effectivement lisible | Session déjà établie, caches du navigateur vides, serveur déjà synchronisé, aller-retour réseau de 80 ms | Moins de 1 s |
| Recherche → première page de résultats | Index complet et chaud, 100 000 messages fictifs, requêtes représentatives | Moins de 300 ms après validation de la recherche |
| Notification IMAP reçue → nouvel objet visible | Onglet actif, Réception surveillée, réseau de référence | Moins de 500 ms. Mesurer aussi le délai en amont de la notification lorsqu’il est observable |

**Protocole reproductible.** Utiliser les mêmes messages, réglages de lecture, navigateur, appareil, taille de fenêtre, région et profil réseau pour le nouveau webmail et les références disponibles, dont Afterlogic et Infomaniak Mail. Figer les versions et consigner le matériel. Tester Chrome ou Firefox de bureau et un téléphone de milieu de gamme. Un profil réseau de comparaison proposé utilise 10 Mbit/s en réception, 2 Mbit/s en envoi et 80 ms d’aller-retour. Préparer des boîtes vides, puis de 20, 10 000 et 100 000 messages avec accents, longues conversations, HTML, dates identiques et pièces volumineuses. Mesurer à part le tout premier rattachement Infomaniak et la progression avant indexation complète.

**Preuves et comparaison.** Effectuer au moins 100 répétitions sur les parcours automatisables, après une phase d’échauffement distincte. Publier médiane, p95, taux d’erreur, volume transféré, nombre de requêtes et taux de cache utile. Conserver les traces anonymisées des cas lents. Une comparaison « plus rapide » désigne un parcours, un concurrent, une configuration et une date. Le microtest local antérieur sur 100 000 messages fictifs donnait 0,084 ms au p95 pour 50 résumés avec curseur et 2,98 ms avec décalage SQL, sur 500 lectures à chaud au milieu du dossier. Cette mesure concerne SQLite seul. Les cibles ci-dessus restent à démontrer sur le produit.

## Confidentialité et qualité de service

**Frontières de confiance.** Le contenu d’un email, d’une pièce ou d’une invitation vient d’un expéditeur tiers. Assainir le HTML et isoler le rendu. Bloquer par défaut les ressources distantes et traiter les images intégrées par leurs références MIME. Un transfert ou une réponse ne doit pas réintroduire de contenu actif. Les pièces sont servies avec vérification d’accès, type et disposition adaptés. Les identifiants IMAP/SMTP sont protégés côté serveur, les sessions se terminent à la déconnexion et les journaux excluent adresses complètes, objets, corps, jetons et mots de passe.

**Accessibilité.** Chaque icône d’action possède un nom accessible et une indication de focus. Le clavier permet de lire, répondre, classer et revenir à la liste, y compris avec les séparateurs ajustables. Les boîtes de dialogue gardent le focus, Échap les ferme selon leur rôle et rend le focus au déclencheur. Les messages d’erreur, opérations en attente et compteurs se comprennent indépendamment de la couleur. Une personne utilisant un lecteur d’écran entend le changement utile sans se faire relire tout le dossier.

**Essais de fiabilité.** Injecter coupure avant et après acceptation d’une action, événement double ou manquant, expiration de session, dossier supprimé sur un autre appareil, arrivée d’un mail pendant une pagination, index en reconstruction, disque plein, stockage navigateur évincé, gros HTML et fichier dépassant la limite. Tester deux comptes dont des dossiers contiennent le même UID. Contrôler qu’une recherche, un lien profond, un téléchargement et une notification ne donnent accès qu’aux données du bon compte. Suivre la perte de brouillon, le double envoi et la fuite intercompte comme anomalies bloquantes de livraison.

## Découpage proposé pour le Kanban

Les intitulés ci-dessous peuvent devenir des cartes indépendantes. Chaque carte livre un comportement observable et cite ses dépendances directes. **Lecture** désigne la première tranche utilisable. **Quotidien** vise le remplacement du webmail pour une boîte. **Parité** ajoute les parcours de confort et l’exploitation avancée. **Option** réclame une décision d’activation. Les lots de synchronisation, de mesure et de mobile commencent dès que leurs prérequis sont disponibles.

### Première lecture de bout en bout

| Carte candidate | Critère de recette | Prérequis |
| --- | --- | --- |
| **F01** · Figer les écrans de référence · *Lecture* | Une planche sur données fictives identifie les états à comparer, le build et les trois formats d’écran. | — |
| **F02** · Préparer les boîtes de recette et le chronométrage · *Lecture* | Des messages reproductibles alimentent IMAP/SMTP de test. Une trace distingue clic, rendu du contenu et confirmation distante. | — |
| **F03** · Configurer le profil Infomaniak · *Lecture* | Un administrateur visualise les deux protocoles sécurisés. Un hôte indisponible affiche un diagnostic et conserve le profil précédent. | — |
| **F04** · Ouvrir et fermer une session de courrier · *Lecture* | Une authentification valide mène à Réception. Une session expirée revient à la connexion. Une seconde identité reçoit uniquement sa boîte. | F03 |
| **F05** · Découvrir les dossiers et leurs rôles · *Lecture* | Envoyés et Corbeille restent reconnus lorsqu’ils portent un libellé différent. Un dossier sans rôle reste consultable. | F02, F04 |
| **F06** · Servir la première page indexée · *Lecture* | Les premiers résumés apparaissent avant la fin de l’import. Une reprise au milieu d’un lot conserve une ligne par message. | F05 |
| **F07** · Construire la vue à trois volets · *Lecture* | Le dossier, les lignes et le lecteur occupent les positions de référence. Un redimensionnement et Retour conservent largeurs et sélection. | F01, F06 |
| **F08** · Lire un message réel depuis sa ligne · *Lecture* | Le clic présente l’expéditeur, l’objet et le corps texte ou HTML assaini. Une réponse tardive de A ne remplace pas B à l’écran. | F07 |

### Navigation et classement

| Carte candidate | Critère de recette | Prérequis |
| --- | --- | --- |
| **L01** · Distinguer aperçu et cases cochées · *Quotidien* | Ouvrir A, cocher B et C, puis marquer comme lus ne modifie que B et C. La sélection partielle de page est annoncée. | F07 |
| **L02** · Paginer et restaurer la position · *Quotidien* | Deux pages successives aux dates identiques forment un ensemble stable. Un retour depuis un autre dossier replace le focus sur la bonne ligne. | F06, F07 |
| **L03** · Traiter une action optimiste · *Quotidien* | Étoile, lu et non lu réagissent dès le clic. Une opération répétée retourne le même résultat et un refus restaure le bon état. | F06, L01 |
| **L04** · Déplacer, copier et archiver · *Quotidien* | Le menu et le glisser-déposer agissent sur le même lot. Un dossier cible absent ou un échec partiel produit un retour précis. | F05, L03 |
| **L05** · Supprimer, restaurer et vider la corbeille · *Quotidien* | Le premier geste déplace dans la corbeille. L’action définitive demande confirmation et cite la portée exacte. | L04 |
| **L06** · Organiser l’arborescence et traiter les indésirables · *Quotidien* | Créer puis renommer un sous-dossier actualise le sélecteur de destination. Sortir un message du spam met à jour les deux compteurs. | F05, L04 |
| **L07** · Parcourir une conversation au clavier · *Parité* | Entrée déplie le fil. Le rang, les non-lus et le message d’origine restent identifiables après navigation. | F08, L02 |
| **L08** · Offrir les actions de lecture secondaires · *Parité* | Les en-têtes, l’EML, l’impression et la vue séparée concernent le message actif. La fermeture retrouve la bonne ligne. | F08 |

### Rédaction et pièces jointes

| Carte candidate | Critère de recette | Prérequis |
| --- | --- | --- |
| **C01** · Composer un premier brouillon · *Quotidien* | Créer un message avec À, Cc et Cci, corriger une adresse et retrouver l’objet et le contenu après sauvegarde manuelle. | F04, F07 |
| **C02** · Retrouver un brouillon après une coupure · *Quotidien* | Le texte saisi sur appareil de confiance survit au rechargement. La reprise synchronise la dernière version et conserve un conflit identifiable. | C01, F05 |
| **C03** · Répondre, répondre à tous et transférer · *Quotidien* | Une réponse utilise Reply-To. Répondre à tous exclut les doublons et respecte les identités. Le fil d’origine s’ouvre au retour. | C01, F08 |
| **C04** · Passer de la réponse rapide à la rédaction complète · *Quotidien* | Le texte rédigé dans le volet droit, le compte et les destinataires sont conservés, y compris après minimisation. | C02, C03 |
| **C05** · Joindre, annuler et reprendre un fichier · *Quotidien* | Le transfert affiche sa progression. Après un échec, le texte reste disponible et le fichier peut être repris ou retiré. | C01 |
| **C06** · Télécharger et prévisualiser une pièce reçue · *Quotidien* | Un fichier autorisé s’ouvre ou se télécharge en flux. Une pièce volumineuse ne retarde pas le premier texte lisible. | F08 |
| **C07** · Soumettre une fois et classer dans Envoyés · *Quotidien* | Deux clics déclenchent une soumission. Un échec de copie dans Envoyés relance le classement seul. | C02, C05 |
| **C08** · Traiter un envoi incertain ou partiellement refusé · *Quotidien* | Une coupure après remise des données rend l’état explicite. Les refus de destinataires sont distingués des acceptations. | C07, F02 |
| **C09** · Personnaliser la rédaction · *Parité* | L’identité, sa signature, le mode texte ou HTML et le raccourci d’envoi respectent le réglage de l’utilisateur. | C03, C07 |

### Synchronisation et vitesse

| Carte candidate | Critère de recette | Prérequis |
| --- | --- | --- |
| **S01** · Mettre la liste et les messages récents en cache privé · *Quotidien* | Une seconde ouverture affiche la liste sans attente réseau. Changer de compte ou se déconnecter empêche le réaffichage de l’ancienne boîte. | F04, F08 |
| **S02** · Recevoir les changements en direct · *Quotidien* | Après réception d’une notification IMAP, l’onglet actif actualise une ligne et son compteur. Trois onglets partagent le travail de synchronisation du compte. | F05, F06 |
| **S03** · Rattraper une interruption du flux · *Quotidien* | Un événement perdu puis dupliqué conduit à la bonne révision. Une révision trop ancienne déclenche un instantané du dossier. | S01, S02 |
| **S04** · Reconstruire un dossier après changement de génération · *Quotidien* | Une nouvelle UIDVALIDITY invalide les anciens identifiants. Une action en attente ne peut atteindre un autre message ayant repris le même UID. | F05, L03, S03 |
| **S05** · Prioriser lecture et préchargement · *Quotidien* | Une demande de lecture passe avant l’historique. Un corps préchargé reste non lu et une pièce lourde attend une action de l’utilisateur. | C06, S01 |
| **S06** · Gérer les limites de stockage et les reprises · *Quotidien* | L’éviction libère d’abord les lectures anciennes. Le mode partagé, un quota navigateur plein et une fermeture avec brouillon local ont un état compréhensible. | C02, S01 |
| **S07** · Vérifier les budgets de latence · *Quotidien* | Le rapport distingue les états chaud, froid serveur prêt et premier rattachement. Il contient p50, p95, erreurs et conditions de mesure. | F02, L03, S01, S02 |
| **S08** · Décider du cache partagé et du hub temps réel · *Parité* | Une comparaison chiffrée motive le maintien du service unique ou l’adoption de Redis ou Mercure, avec portée privée et reprise testée. | S03, S07 |

### Recherche et comptes

| Carte candidate | Critère de recette | Prérequis |
| --- | --- | --- |
| **R01** · Chercher dans le dossier actif · *Quotidien* | Une recherche par objet, expéditeur ou texte renvoie des résultats paginés du bon compte. Une ancienne réponse ne remplace pas la recherche actuelle. | F06, L02 |
| **R02** · Combiner les critères avancés · *Quotidien* | Dates, destinataire, pièce jointe et dossier étendu se combinent. La recherche signale un index partiel et explique le périmètre de zéro résultat. | R01 |
| **R03** · Créer un contact et suggérer son adresse · *Quotidien* | Une fiche créée depuis un expéditeur est retrouvée dans le composeur. Un homonyme présente son adresse avant la sélection. | C01, F08 |
| **R04** · Rattacher une deuxième boîte · *Parité* | Deux boîtes ayant les mêmes noms de dossiers et les mêmes UID gardent des listes, des événements et des brouillons indépendants. | F04, C02, S03 |
| **R05** · Consulter une réception unifiée · *Parité* | Chaque ligne indique sa boîte. Répondre et archiver utilisent l’identité et le dossier du bon compte. | R04, C03, L04 |
| **R06** · Gérer groupes et échanges de contacts · *Parité* | Un import avec doublons et lignes invalides rend un bilan. Un export ou une suggestion de groupe respecte les contacts sélectionnés. | R03 |

### Mobile et exploitation

| Carte candidate | Critère de recette | Prérequis |
| --- | --- | --- |
| **A01** · Lire et répondre sur téléphone · *Quotidien* | Le tiroir mène à la liste, puis à la lecture et à la réponse. Retour restaure le rang et le clavier virtuel laisse les commandes accessibles. | F07, C03 |
| **A02** · Travailler au clavier et au lecteur d’écran · *Quotidien* | Le parcours connexion, recherche, lecture, réponse et archivage est réalisable au clavier. S et R tapés dans un champ restent du texte. | A01, R01, L04 |
| **A03** · Régler les préférences et les modules · *Parité* | Une politique verticale fixe supprime le choix personnel de disposition. Une signature ou une langue modifiée est appliquée au bon profil. | F03, C09 |
| **A04** · Administrer les comptes et diagnostiquer la synchronisation · *Parité* | L’administrateur constate un retard, relance un dossier et vérifie le retour à jour à partir de métriques dépourvues de contenu privé. | F03, S03 |
| **A05** · Restaurer une sauvegarde et migrer le schéma · *Parité* | Sur une instance vierge, rétablir préférences, brouillons et file durable. Une commande SMTP déjà acceptée garde son état. | C08, S04 |
| **A06** · Valider l’isolation et la compatibilité · *Parité* | Deux comptes, plusieurs navigateurs et deux tailles d’écran passent les scénarios de session, images distantes, pièces, liens profonds et caches. | A01, C06, R04 |
| **A07** · Comparer le produit livré aux références · *Parité* | Avec une boîte d’essai Infomaniak et un jeu comparable, publier les parcours mesurés, les écarts d’ergonomie et les limites constatées. | A02, A04, S07, R02 |

### Extensions de parité

| Carte candidate | Critère de recette | Prérequis |
| --- | --- | --- |
| **X01** · Gérer calendriers, invitations et fuseaux · *Option* | Une invitation ouverte depuis le courrier reçoit une réponse avec le bon compte. Une récurrence traverse le changement d’heure. | C07, R03 |
| **X02** · Relier documents personnels et pièces jointes · *Option* | Enregistrer une pièce dans Fichiers puis la joindre à un brouillon. La limite d’espace s’applique au compte concerné. | C05, C06 |
| **X03** · Gérer tâches et notes · *Option* | Créer, modifier puis retrouver une tâche et une note. Revenir vers la messagerie conserve le dossier courant. | F07, A03 |
| **X04** · Configurer les règles serveur · *Option* | Après validation de la capacité du fournisseur, une règle de recette classe le message prévu et l’interface reflète son ordre et son activation. | F03, L04 |
| **X05** · Activer la synchronisation des carnets et calendriers · *Option* | Tester CardDAV et CalDAV avec des comptes d’essai et une modification concurrente. L’interface indique le statut par collection. | R06, X01 |
| **X06** · Évaluer les options avancées de rédaction · *Option* | Pour chaque option retenue parmi modèles, programmation d’envoi et OpenPGP, spécifier séparément l’état visible, les données à conserver et la récupération. | C08, A03 |

## Exemple de carte prête à développer

**L04 · Archiver une sélection et conserver la place de lecture.** En tant qu’utilisateur qui traite Réception au clavier, je veux que les messages archivés quittent la liste dès mon action, pour enchaîner sur le message suivant tout en pouvant vérifier que le classement a réussi. Cette carte couvre l’archivage. La copie et le glisser-déposer peuvent devenir des cartes sœurs du même lot.

| Situation donnée | Action | Résultat vérifiable |
| --- | --- | --- |
| A ouvert, B et C cochés, Archives associé | Lancer Archiver | B et C quittent immédiatement la liste. A reste à l’écran. Les compteurs reflètent les non-lus déplacés et un état transitoire est visible. |
| Un seul message ouvert et aucune case cochée | Archiver le message ouvert | La ligne suivante prend le focus. Si le dossier devient vide, son état vide reçoit le focus et le raccourci suivant reste utilisable. |
| Infomaniak confirme B et refuse C | Attendre la synchronisation | B est confirmé dans Archives. C est rétabli dans Réception. Seul C est proposé à la reprise. |
| Une réponse HTTP se perd puis la commande est rejouée | Renvoyer le même identifiant d’opération | Le backend rend l’état connu. Une reconnexion SSE replace les deux onglets sur la même révision. |
| Mesure sur boîte chaude | Chronométrer le clic jusqu’à la disparition de la ligne | Moins de 50 ms au p95 pour la réaction locale. Le délai de confirmation IMAP est enregistré séparément. |

**Champs de la future carte.** Indiquer le compte de recette, l’état initial, le contrôle d’interface, les données touchées, le chemin nominal, le refus distant, le fonctionnement à froid ou hors ligne, la cible de performance et les dépendances. Pour L04, la conception dépend de F05 et L03. Le scénario de reprise entre deux onglets se vérifie avec S03. Une capture avant et après, une trace de latence et un résultat de lecture dans le dossier Archives constituent les preuves attendues.

## Jalons et critères de livraison

**Lecture.** Une installation neuve peut rattacher une boîte Infomaniak de test, afficher ses dossiers et ouvrir un message dans les trois volets. Le premier écran reste utilisable pendant le chargement de l’historique. La géométrie de bureau, la navigation et la mesure de ce premier parcours sont disponibles ensemble.

**Quotidien.** Sur une boîte de recette, traiter une série de messages depuis le clavier et depuis un téléphone, répondre avec une pièce, reprendre un brouillon, retrouver un message par la recherche, archiver puis restaurer. Répéter avec une coupure et un déplacement effectué depuis un second appareil. Le statut final doit être vérifiable chez Infomaniak. Les traces indiquent quels budgets du tableau de performance sont atteints.

**Parité.** Un utilisateur Afterlogic peut reprendre ses habitudes de sélection, conversations, préférences, comptes liés et actions secondaires. Une restauration sur instance vierge et un test d’isolation à deux comptes précèdent l’ouverture à plusieurs utilisateurs. Chaque écart visible encore accepté est daté et rattaché à un parcours.

**Extensions.** Pour chaque module retenu, préciser qui s’en sert, quelle donnée fait autorité, comment le désactiver, quel parcours de courrier il enrichit et comment sa latence est mesurée. Le premier lancement demeure utilisable lorsque ce module est indisponible.

**Une carte est terminée** quand son scénario nominal et son échec principal sont démontrés sur données de test, que la sélection et le focus restent cohérents, que les changements de données ont une issue de reprise et que la mesure prévue est jointe. Les cartes de surface reçoivent une comparaison avec la référence. Les cartes de fond reçoivent une observation utilisateur ou une mesure qui démontre leur effet.

**Arbitrages encore ouverts.** Choisir avec le propriétaire la politique d’appareil partagé et la rétention des corps locaux, le dossier d’archivage par défaut, le volume de boîtes et de connexions à dimensionner, les modules de collaboration réellement utilisés et le jeu de droits sur les ressources visuelles. Les cibles initiales correspondent à une instance personnelle. Un test authentifié sur une boîte Infomaniak dédiée établira les capacités annoncées après connexion et les latences réelles du fournisseur.
