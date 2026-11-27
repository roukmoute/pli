Feature: Lire une conversation et retrouver sa place

  Scenario Outline: Ouvrir une lettre illustrée puis retrouver sa ligne pendant la mise à jour
    Given le profil du banc est en service
    When je précharge puis lis un fil illustré dans le navigateur
    Then le préchargement marque <precharges> messages lus
    And l’ouverture marque <ouverts> messages lus
    And les images distantes contactées avant mon choix sont au nombre de <images>
    And le corps demandé depuis une autre boîte répond avec le code <code>
    And le nombre de requêtes lancées avant de retrouver la liste est <requetes>

    Examples:
      | precharges | ouverts | images | code | requetes |
      | 0          | 1       | 0      | 404  | 0        |
