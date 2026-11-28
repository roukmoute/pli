Feature: Déplacer et copier vers un dossier

  Scenario Outline: Ranger des fils par le menu puis par le glisser, en copier un
    Given le profil du banc est en service
    When je déplace et copie des fils par le menu puis par le glisser dans le navigateur
    Then le menu et le glisser produisent <memes> même opération sur leur lot
    And la copie laisse le fil dans <dossiers> dossiers de la boîte
    And le dossier disparu est annoncé pour <refuses> fil en attente d’un autre choix
    And le sélecteur propose <etrangers> dossier d’un autre compte
    And le dernier choix porte à <ranges> le nombre de lettres du dossier

    Examples:
      | memes | dossiers | refuses | etrangers | ranges |
      | 1     | 2        | 1       | 0         | 5      |
