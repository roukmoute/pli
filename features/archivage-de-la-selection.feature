Feature: Archiver une sélection et garder la place de lecture

  Scenario Outline: Archiver deux fils après avoir consulté le premier
    Given une boîte dont la messagerie laisse une lettre sur place au premier archivage est rattachée
    When j’archive deux fils sélectionnés après avoir consulté le premier dans le navigateur
    Then la liste perd <partis> fils avant toute réponse
    And le fil consulté reste avec <lus> lettre lue en Réception
    And la messagerie range <confirmes> lettre dans Archives
    And le refus ramène <revenus> fil sélectionné en Réception
    And l’archivage rejoué rend <identiques> résultat identique
    And la reprise porte à <archives> le nombre de lettres dans Archives
    And la lecture du premier fil a déplacé <deplaces> lettre

    Examples:
      | partis | lus | confirmes | revenus | identiques | archives | deplaces |
      | 2      | 1   | 1         | 1       | 1          | 2        | 0        |
