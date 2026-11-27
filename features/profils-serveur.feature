Feature: Configurer le serveur de messagerie

  Scenario Outline: Proposer les réglages sécurisés au premier démarrage
    When je consulte les serveurs de messagerie
    Then le nombre de serveurs configurés est <nombre>
    And la réception proposée est <reception>
    And la soumission proposée est <soumission>

    Examples:
      | nombre | reception                                | soumission                          |
      | 0      | mail.infomaniak.com 993 TLS implicite     | mail.infomaniak.com 587 STARTTLS     |

  Scenario Outline: Réserver les réglages à l’administrateur
    When je consulte les serveurs sans jeton
    Then le service répond avec le code <code>

    Examples:
      | code |
      | 401  |

  Scenario Outline: Garder le profil en service après un diagnostic en échec
    Given le profil du banc est en service
    When je remplace sa réception par un hôte inconnu et sa soumission par un port fermé
    Then le service répond avec le code <code>
    And la résolution du serveur de réception indique <dns>
    And la connexion au serveur de soumission indique <connexion>
    And la négociation TLS de soumission indique <tls>
    And l’authentification de réception indique <auth>
    When je consulte les serveurs de messagerie
    Then le profil en service est <profil>

    Examples:
      | code | dns   | connexion | tls       | auth      | profil    |
      | 422  | échec | échec     | non testé | non testé | Banc local |
