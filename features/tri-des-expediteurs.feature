Feature: Trier les arrivées pendant l’import de la boîte

  Scenario Outline: Recevoir deux lettres pendant un lot interrompu puis retrouver le tri
    Given une boîte contenant deux anciennes lettres est rattachée
    When deux nouvelles lettres du même expéditeur arrivent pendant la lecture du premier lot
    Then le service répond avec le code <http>
    And le nombre d’expéditeurs à trier est <expediteurs>
    And le nombre de lettres de cet expéditeur est <lettres>
    And l’adresse présentée au tri est <adresse>
    And le nom présenté au tri est <nom>
    And le dernier objet présenté est <objet>
    And le dernier extrait présenté est <extrait>
    When le service redémarre au milieu du lot et reprend l’import
    Then le nombre d’expéditeurs à trier est <expediteurs>
    And le nombre de lettres de cet expéditeur est <lettres>
    And le nombre de lettres importées dans la boîte est <total>
    And le nombre de lettres admises en Réception est <historique>
    And le nombre de lettres non lues sur le serveur de courrier est <non_lues>

    Examples:
      | http | expediteurs | lettres | adresse             | nom       | objet     | extrait                      | total | historique | non_lues |
      | 200  | 1           | 2       | atelier@recette.test | L’Atelier | Arrivée B | Lettre de l’atelier. Arrivée B. | 4     | 2          | 4        |
