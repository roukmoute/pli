Feature: Parcourir les pages de fils sans perdre sa place

  Scenario Outline: Retrouver sa ligne en troisième page puis découvrir une arrivée
    Given le profil du banc est en service
    When je charge les pages de Réception, change de dossier puis reçois une lettre dans le navigateur
    Then les pages chargées présentent <fils> fils sans doublon
    And le retour replace le focus sur une ligne de la page <page>
    And l’arrivée déplace la ligne survolée de <ecart> pixel
    And l’accès aux nouveautés annonce <arrivees> fil

    Examples:
      | fils | page | ecart | arrivees |
      | 20   | 3    | 0     | 1        |

  Scenario Outline: Garder des lots d’import indépendants de la taille de page
    Given je rattache une boîte dont le second lot est suspendu
    When je consulte la réception pendant l’import
    Then le nombre de fils en Réception est <importes>
    And la page propose <affiches> fils

    Examples:
      | importes | affiches |
      | 50       | 4        |
