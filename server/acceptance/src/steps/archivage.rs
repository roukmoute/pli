use super::{
    Example, Problem, World, example_value, expect,
    navigation::{commande_navigation, observer},
    parameter,
};

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/archivage.mjs");
const BOITE: &str =
    "une boîte dont la messagerie laisse une lettre sur place au premier archivage est rattachée";
const GESTE: &str =
    "j’archive deux fils sélectionnés après avoir consulté le premier dans le navigateur";
// Début et fin de chaque étape, autour de la valeur attendue, et mesure rendue par la sonde.
const MESURES: [(&str, &str, &str); 7] = [
    ("la liste perd ", " fils avant toute réponse", "immediats"),
    (
        "le fil consulté reste avec ",
        " lettre lue en Réception",
        "consultes",
    ),
    ("la messagerie range ", " lettre dans Archives", "confirmes"),
    (
        "le refus ramène ",
        " fil sélectionné en Réception",
        "revenus",
    ),
    (
        "l’archivage rejoué rend ",
        " résultat identique",
        "identiques",
    ),
    (
        "la reprise porte à ",
        " le nombre de lettres dans Archives",
        "archives",
    ),
    (
        "la lecture du premier fil a déplacé ",
        " lettre",
        "deplaces",
    ),
];

impl World {
    pub(super) async fn archivage_step(
        &mut self,
        texte: &str,
        exemple: &Example,
    ) -> Result<bool, Problem> {
        if texte == BOITE {
            // Le relais du banc retire la dernière lettre du premier déplacement demandé.
            self.attach_test_mailbox(&["--archivage"]).await?;
            return Ok(true);
        }
        if texte == GESTE {
            let boite = self
                .screener_bench
                .as_ref()
                .ok_or_else(|| Problem::infra("Boîte d’archivage absente."))?;
            let mut sonde = commande_navigation(SCRIPT, self.address);
            sonde.arg(&boite.address);
            self.lecture = observer(&mut sonde).await?;
            return Ok(true);
        }
        for (debut, fin, mesure) in MESURES {
            if let Some(cle) = texte.strip_suffix(fin).and_then(|s| parameter(s, debut)) {
                let valeur = self
                    .lecture
                    .get(mesure)
                    .ok_or_else(|| Problem::Step("Observation d’archivage absente.".into()))?;
                expect(valeur.to_string().as_str(), example_value(exemple, cle)?)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exige_chaque_mesure_de_l_archivage() {
        let mut world = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        world.response = Some((reqwest::StatusCode::OK, "{}".into()));
        let attendu = |valeur: &str| Example::from([("n".into(), valeur.into())]);
        for (debut, fin, mesure) in MESURES {
            let etape = format!("{debut}<n>{fin}");
            assert!(world.step(&etape, &attendu("8")).await.is_err(), "{etape}");
            world.lecture.insert(mesure.into(), 8);
            world.step(&etape, &attendu("8")).await.unwrap();
            assert!(world.step(&etape, &attendu("9")).await.is_err(), "{etape}");
            world.lecture.clear();
        }
        let autre = world.archivage_step("autre étape", &attendu("8")).await;
        assert!(!autre.unwrap());
        // Sans base de recette ni boîte rattachée, aucun des deux gestes n’aboutit.
        for geste in [BOITE, GESTE] {
            assert!(world.archivage_step(geste, &attendu("8")).await.is_err());
        }
    }
}
