Feature: Parcourir la réception indexée

  Scenario Outline: Reprendre la lecture à la page suivante
    Given le profil du banc est en service
    And Alice ouvre sa session
    When je consulte deux pages consécutives de ma réception
    Then le service répond avec le code <http>
    And chaque page contient <taille> fils
    And les deux pages présentent <distincts> fils distincts
    And la réception indique <attente> expéditeurs en attente

    Examples:
      | http | taille | distincts | attente |
      | 200  | 2      | 4         | 0       |

  Scenario Outline: Lire le premier lot puis retrouver le compteur après reprise
    Given je rattache une boîte dont le second lot est suspendu
    When je consulte la réception pendant l’import
    Then le nombre de fils en Réception est <partiels>
    And le total prévu par l’import est <total>
    And la page propose <affiches> fils
    When le service reprend le lot suspendu
    Then le nombre de fils en Réception est <total>
    And le nombre de lettres indexées est <total>

    Examples:
      | partiels | total | affiches |
      | 50       | 51    | 20       |
