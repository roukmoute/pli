Feature: Sélectionner des fils et marquer leur courrier

  Scenario Outline: Ouvrir un fil puis agir sur deux autres depuis leurs avatars
    Given le profil du banc est en service
    When je reviens du premier fil, sélectionne les deux autres et change leur état dans le navigateur
    Then la sélection partielle annonce <selection> fils
    And le lot marque <lus> messages lus dans la boîte
    And les autres cibles de la ligne ouvrent <ouverts> messages
    And la boîte étrangère refuse <refuses> identifiant

    Examples:
      | selection | lus | ouverts | refuses |
      | 2         | 2   | 0       | 1       |
