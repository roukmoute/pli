use super::{Example, Problem, World, example_value, expect, parameter};
use serde::{Deserialize, de::DeserializeOwned};
use std::net::SocketAddr;
use tokio::process::Command;

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/navigation.mjs");

pub(super) fn commande_navigation(script: &str, adresse: SocketAddr) -> Command {
    let mut commande = Command::new("node");
    commande
        .arg(script)
        .arg(format!("http://{adresse}"))
        .kill_on_drop(true);
    commande
}

pub(super) async fn observer<T: DeserializeOwned>(commande: &mut Command) -> Result<T, Problem> {
    let sortie = commande.output().await.map_err(Problem::infra)?;
    if !sortie.status.success() {
        return Err(Problem::Step(
            String::from_utf8_lossy(&sortie.stderr).into_owned(),
        ));
    }
    serde_json::from_slice(&sortie.stdout).map_err(Problem::infra)
}

#[derive(Deserialize)]
pub(super) struct Navigation {
    retours: u32,
    #[serde(rename = "compteurVide")]
    attente: u32,
}

impl World {
    pub(super) async fn navigation_step(
        &mut self,
        text: &str,
        row: &Example,
    ) -> Result<bool, Problem> {
        if text == "je parcours le menu et reviens en Réception dans le navigateur" {
            self.navigation = Some(observer(&mut commande_navigation(SCRIPT, self.address)).await?);
            return Ok(true);
        }
        let Some(nav) = &self.navigation else {
            return Ok(false);
        };
        let (cle, valeur) = if let Some(cle) = parameter(
            text.trim_end_matches(" formats"),
            "le fil et le défilement sont retrouvés sur ",
        ) {
            (cle, nav.retours)
        } else if let Some(cle) = parameter(
            text.trim_end_matches(" expéditeurs à trier"),
            "la boîte vide affiche ",
        ) {
            (cle, nav.attente)
        } else {
            return Ok(false);
        };
        expect(valeur.to_string().as_str(), example_value(row, cle)?)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrompt_la_sonde_avec_le_scenario() {
        let commande = commande_navigation(SCRIPT, "127.0.0.1:1234".parse().unwrap());
        assert!(commande.get_kill_on_drop());
    }
    #[tokio::test]
    async fn verifie_les_deux_observations_recues_du_navigateur() {
        let mut world = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        world.response = Some((reqwest::StatusCode::OK, "{}".into()));
        let mut row = Example::new();
        let etat = "la boîte vide affiche <n> expéditeurs à trier";
        assert!(!world.navigation_step(etat, &row).await.unwrap());
        world.navigation = Some(Navigation {
            retours: 3,
            attente: 0,
        });
        for (texte, n) in [
            (etat, "0"),
            (
                "le fil et le défilement sont retrouvés sur <n> formats",
                "3",
            ),
        ] {
            row.insert("n".into(), n.into());
            world.step(texte, &row).await.unwrap();
            row.insert("n".into(), "9".into());
            assert!(world.step(texte, &row).await.is_err());
        }
        assert!(
            !world
                .navigation_step("une autre étape", &row)
                .await
                .unwrap()
        );
    }
}
