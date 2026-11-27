use super::{
    Example, Problem, World, example_value, expect,
    navigation::{commande_navigation, observer},
};
use serde::Deserialize;

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/decisions.mjs");

#[derive(Deserialize)]
pub(super) struct Observations {
    formats: u32,
    flux: u32,
    conflit: u32,
    historique: u32,
    justificatifs: u32,
}

impl World {
    pub(super) async fn decisions_step(
        &mut self,
        texte: &str,
        exemple: &Example,
    ) -> Result<bool, Problem> {
        if texte == "je trie l’Atelier dans deux onglets et révise le refus depuis mon avatar" {
            let banc = self
                .screener_bench
                .as_ref()
                .ok_or_else(|| Problem::infra("Boîte de tri absente."))?;
            self.decisions =
                Some(observer(commande_navigation(SCRIPT, self.address).arg(&banc.address)).await?);
            return Ok(true);
        }
        let (Some(mesures), Some((prefixe, cle))) = (&self.decisions, texte.split_once('<')) else {
            return Ok(false);
        };
        let valeur = match prefixe {
            "le Screener tient dans les formats de référence au nombre de " => mesures.formats,
            "les fils visibles dans Flux pendant l’enregistrement sont au nombre de " => {
                mesures.flux
            }
            "le code du choix concurrent affiché au deuxième onglet est " => mesures.conflit,
            "le nombre de lettres accessibles après le refus dans l’historique est " => {
                mesures.historique
            }
            "le nombre de fils rétablis dans Justificatifs après rechargement est " => {
                mesures.justificatifs
            }
            _ => return Ok(false),
        };
        expect(
            valeur.to_string().as_str(),
            example_value(exemple, cle.trim_end_matches('>'))?,
        )?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn compare_chaque_observation_et_ignore_les_etapes_des_autres_ecrans() {
        let mut monde = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        monde.response = Some((reqwest::StatusCode::OK, "{}".into()));
        let mut exemple = Example::new();
        let mesures = [
            (
                "le Screener tient dans les formats de référence au nombre de <n>",
                3,
            ),
            (
                "les fils visibles dans Flux pendant l’enregistrement sont au nombre de <n>",
                4,
            ),
            (
                "le code du choix concurrent affiché au deuxième onglet est <n>",
                409,
            ),
            (
                "le nombre de lettres accessibles après le refus dans l’historique est <n>",
                5,
            ),
            (
                "le nombre de fils rétablis dans Justificatifs après rechargement est <n>",
                6,
            ),
        ];
        assert!(
            !monde
                .decisions_step("une observation <n>", &exemple)
                .await
                .unwrap()
        );
        assert!(
            monde
                .decisions_step(
                    "je trie l’Atelier dans deux onglets et révise le refus depuis mon avatar",
                    &exemple
                )
                .await
                .is_err()
        );
        monde.decisions = Some(Observations {
            formats: 3,
            flux: 4,
            conflit: 409,
            historique: 5,
            justificatifs: 6,
        });
        for (etape, observe) in mesures {
            exemple.insert("n".into(), observe.to_string());
            monde.step(etape, &exemple).await.unwrap();
            exemple.insert("n".into(), "99".into());
            assert!(monde.step(etape, &exemple).await.is_err());
        }
        assert!(
            !monde
                .decisions_step("autre écran <n>", &exemple)
                .await
                .unwrap()
        );
    }
}
