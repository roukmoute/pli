use std::collections::BTreeMap;

use super::{
    Example, Problem, World, example_value, expect,
    navigation::{commande_navigation, observer},
    parameter,
};

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/lecture.mjs");

impl World {
    pub(super) async fn lecture_step(
        &mut self,
        texte: &str,
        exemple: &Example,
    ) -> Result<bool, Problem> {
        if texte == "je précharge puis lis un fil illustré dans le navigateur" {
            self.lecture =
                observer::<BTreeMap<String, u32>>(&mut commande_navigation(SCRIPT, self.address))
                    .await?;
            return Ok(true);
        }
        for (prefixe, suffixe, mesure) in [
            ("le préchargement marque ", " messages lus", "prechargement"),
            ("l’ouverture marque ", " messages lus", "ouverture"),
            (
                "le nombre de requêtes lancées avant de retrouver la liste est ",
                "",
                "requetes_avant_rendu",
            ),
            (
                "les images distantes contactées avant mon choix sont au nombre de ",
                "",
                "distantes",
            ),
            (
                "le corps demandé depuis une autre boîte répond avec le code ",
                "",
                "autre",
            ),
        ] {
            if let Some(cle) = texte
                .strip_suffix(suffixe)
                .and_then(|s| parameter(s, prefixe))
            {
                let valeur = self
                    .lecture
                    .get(mesure)
                    .ok_or_else(|| Problem::Step("Observation de lecture absente.".into()))?;
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
    async fn exige_la_mesure_de_chaque_geste() {
        let mut world = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        for (etape, champ) in [
            ("le préchargement marque <n> messages lus", "prechargement"),
            ("l’ouverture marque <n> messages lus", "ouverture"),
            (
                "le nombre de requêtes lancées avant de retrouver la liste est <n>",
                "requetes_avant_rendu",
            ),
            (
                "les images distantes contactées avant mon choix sont au nombre de <n>",
                "distantes",
            ),
            (
                "le corps demandé depuis une autre boîte répond avec le code <n>",
                "autre",
            ),
        ] {
            let oui = Example::from([("n".into(), "8".into())]);
            assert!(world.lecture_step(etape, &oui).await.is_err());
            world.lecture.insert(champ.into(), 8);
            assert!(world.lecture_step(etape, &oui).await.unwrap());
            assert!(
                world
                    .lecture_step(etape, &Example::from([("n".into(), "9".into())]))
                    .await
                    .is_err()
            );
        }
        assert!(!world.lecture_step("autre", &Example::new()).await.unwrap());
    }
}
