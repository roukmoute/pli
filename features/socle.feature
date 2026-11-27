Feature: Connaître la disponibilité du webmail

  Scenario Outline: Le webmail annonce qu’il est disponible
    When je consulte la disponibilité du webmail
    Then le service répond avec le code <code>
    And l’état annoncé est <etat>

    Examples:
      | code | etat |
      | 200  | ok   |
