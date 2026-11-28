Feature: Supprimer, restaurer et vider la corbeille

  Scenario Outline: Jeter des fils, en restaurer un, puis effacer les autres
    Given le profil du banc est en service
    When je supprime, restaure puis vide la corbeille dans le navigateur
    Then la suppression retire <partis> fil avant toute réponse et sans confirmation
    And la restauration ramène <revenus> lettre en Réception
    And Réception compte alors <reception> fils
    And la corbeille compte alors <jetes> fil
    And le vidage annulé garde <gardes> lettres dans la corbeille
    And la suppression définitive épargne <epargnes> lettre non ciblée
    And le vidage confirmé laisse <restes> lettre dans la corbeille

    Examples:
      | partis | revenus | reception | jetes | gardes | epargnes | restes |
      | 1      | 1       | 4         | 0     | 2      | 1        | 0      |
