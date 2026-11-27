Feature: Réviser le tri personnel d’une adresse

  Scenario Outline: Appliquer le choix au courrier ancien et nouveau dans deux onglets
    Given une boîte contenant deux anciennes lettres est rattachée
    When deux nouvelles lettres du même expéditeur arrivent pendant la lecture du premier lot
    And le service redémarre au milieu du lot et reprend l’import
    And je trie l’Atelier dans deux onglets et révise le refus depuis mon avatar
    Then le Screener tient dans les formats de référence au nombre de <formats>
    And les fils visibles dans Flux pendant l’enregistrement sont au nombre de <admis>
    And le code du choix concurrent affiché au deuxième onglet est <conflit>
    And le nombre de lettres accessibles après le refus dans l’historique est <archives>
    And le nombre de fils rétablis dans Justificatifs après rechargement est <retablis>
    And le nombre de lettres non lues sur le serveur de courrier est <non_lues>

    Examples:
      | formats | admis | conflit | archives | retablis | non_lues |
      | 3       | 4     | 409     | 4        | 4        | 4        |
