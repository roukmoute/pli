Feature: Découvrir les dossiers et enregistrer le repère de réception

  Background:
    Given le profil du banc est en service
    And Alice ouvre sa session

  Scenario Outline: Retrouver les dossiers de la boîte malgré leurs libellés étrangers
    When je consulte les dossiers de ma boîte
    Then le service répond avec le code <http>
    And le dossier des envoyés est <envoyes>
    And le dossier de la corbeille est <corbeille>
    And les dossiers personnels sont <personnels>

    Examples:
      | http | envoyes  | corbeille  | personnels |
      | 200  | Gesendet | Papierkorb | Atelier    |

  Scenario Outline: Garder le point de départ du tri au redémarrage
    When je consulte les dossiers de ma boîte
    Then le prochain numéro de réception au rattachement est <prochain>
    And le nombre de messages déjà importés est <importes>
    When le service redémarre et je retrouve mes dossiers
    Then le service répond avec le code <http>
    And le repère de réception depuis le rattachement est <repere>

    Examples:
      | prochain | importes | http | repere    |
      | 21       | 0        | 200  | identique |

  Scenario Outline: Consulter uniquement les dossiers du compte ouvert
    Given Bob ouvre sa propre session
    When Bob consulte les dossiers en indiquant le compte d’Alice
    Then le service répond avec le code <http>
    And les chemins des dossiers consultables sont <chemins>

    Examples:
      | http | chemins |
      | 200  | INBOX   |
