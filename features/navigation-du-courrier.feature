Feature: Revenir dans sa boîte de courrier

  Scenario Outline: Retrouver le fil et sa section pendant une mise à jour
    Given le profil du banc est en service
    When je parcours le menu et reviens en Réception dans le navigateur
    Then le fil et le défilement sont retrouvés sur <formats> formats
    And la boîte vide affiche <attente> expéditeurs à trier

    Examples:
      | formats | attente |
      | 3       | 0       |
