Feature: Ouvrir et fermer une session de courrier

  Scenario Outline: Ouvrir la boîte d’Alice et retrouver sa session
    Given le profil du banc est en service
    When Alice ouvre sa session
    And je consulte ma session
    Then le service répond avec le code <http>
    And la boîte ouverte est <adresse>
    And les seules informations de session sont <champs>

    Examples:
      | http | adresse            | champs                        |
      | 200  | alice@recette.test | account_id,address,expires_at |

  Scenario Outline: Refuser un secret incorrect
    Given le profil du banc est en service
    When Alice présente un mauvais secret
    Then le service répond avec le code <refus>
    And le motif annoncé est <identification>

    Examples:
      | refus | identification        |
      | 401   | identifiants_invalides |

  Scenario Outline: Annoncer une panne du serveur de courrier
    Given le serveur de réception vient de devenir inaccessible
    When Alice ouvre sa session
    Then le service répond avec le code <http>
    And le motif annoncé est <motif>

    Examples:
      | http | motif                |
      | 503  | service_indisponible |

  Scenario Outline: Fermer une session sur le serveur
    Given le profil du banc est en service
    And Alice ouvre sa session
    When je ferme ma session
    Then le service répond avec le code <fermeture>
    When je consulte ma session avec mon ancien cookie
    Then le service répond avec le code <refus>

    Examples:
      | fermeture | refus |
      | 204       | 401   |

  Scenario Outline: Laisser expirer une session ouverte
    Given le profil du banc est en service
    And Alice ouvre sa session
    When je consulte ma session après son expiration
    Then le service répond avec le code <refus>
    And le motif annoncé est <motif>

    Examples:
      | refus | motif          |
      | 401   | session_expiree |

  Scenario Outline: Garder les boîtes d’Alice et de Bob séparées
    Given le profil du banc est en service
    And Alice ouvre sa session
    And Bob ouvre sa propre session
    When Bob demande la session en indiquant le compte d’Alice
    Then le service répond avec le code <http>
    And la boîte ouverte est <adresse>

    Examples:
      | http | adresse          |
      | 200  | bob@recette.test |

  Scenario Outline: Protéger les identifiants après une connexion
    Given le profil du banc est en service
    When Alice ouvre sa session
    Then le service répond avec le code <http>
    And le secret en clair dans les fichiers de la boîte est <stockage>
    And le secret dans le journal est <secret>
    And l’adresse complète dans le journal est <adresse>

    Examples:
      | http | stockage | secret | adresse |
      | 200  | absent   | absent | absente |
