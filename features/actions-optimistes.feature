Feature: Agir sur un fil sans attendre la messagerie

  Scenario Outline: Étoiler, marquer lu puis non lu pendant que les réponses sont retenues
    Given le profil du banc est en service
    When j’agis sur un fil pendant que les réponses sont retenues dans le navigateur
    Then la liste montre <gestes> gestes avant toute réponse
    And lu puis non lu laisse <non_lus> message non lu dans la boîte
    And l’opération rejouée rend <identiques> résultat identique
    And l’opération d’une autre boîte est refusée <refusees> fois
    And le refus de la messagerie rétablit <retablies> étoile

    Examples:
      | gestes | non_lus | identiques | refusees | retablies |
      | 3      | 1       | 1          | 1        | 1         |
