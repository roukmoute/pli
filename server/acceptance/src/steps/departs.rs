use super::{
    Example, Problem, World, example_value, expect,
    navigation::{commande_navigation, observer},
    parameter,
};

const ARCHIVAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/archivage.mjs");
const CORBEILLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/corbeille.mjs");
const RANGEMENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/rangement.mjs");
const BOITE: &str =
    "une boîte dont la messagerie laisse une lettre sur place au premier archivage est rattachée";
const GESTE: &str =
    "j’archive deux fils sélectionnés après avoir consulté le premier dans le navigateur";
const VIDAGE: &str = "je supprime, restaure puis vide la corbeille dans le navigateur";
// Chaque parcours du navigateur et sa sonde.
const PARCOURS: [(&str, &str); 2] = [
    (VIDAGE, CORBEILLE),
    (
        "je déplace et copie des fils par le menu puis par le glisser dans le navigateur",
        RANGEMENT,
    ),
];
// Début et fin de chaque étape, autour de la valeur attendue, et mesure rendue par la sonde.
const MESURES: [(&str, &str, &str); 19] = [
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
    (
        "la suppression retire ",
        " fil avant toute réponse et sans confirmation",
        "immediats",
    ),
    ("la restauration ramène ", " lettre en Réception", "revenus"),
    ("Réception compte alors ", " fils", "reception"),
    ("la corbeille compte alors ", " fil", "restants"),
    (
        "le vidage annulé garde ",
        " lettres dans la corbeille",
        "gardes",
    ),
    (
        "la suppression définitive épargne ",
        " lettre non ciblée",
        "epargnes",
    ),
    (
        "le vidage confirmé laisse ",
        " lettre dans la corbeille",
        "videe",
    ),
    (
        "le menu et le glisser produisent ",
        " même opération sur leur lot",
        "memes",
    ),
    (
        "la copie laisse le fil dans ",
        " dossiers de la boîte",
        "copies",
    ),
    (
        "le dossier disparu est annoncé pour ",
        " fil en attente d’un autre choix",
        "annonces",
    ),
    (
        "le sélecteur propose ",
        " dossier d’un autre compte",
        "etrangers",
    ),
    (
        "le dernier choix porte à ",
        " le nombre de lettres du dossier",
        "ranges",
    ),
];

impl World {
    pub(super) async fn depart_step(
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
            let mut sonde = commande_navigation(ARCHIVAGE, self.address);
            sonde.arg(&boite.address);
            self.lecture = observer(&mut sonde).await?;
            return Ok(true);
        }
        for (parcours, sonde) in PARCOURS {
            if texte == parcours {
                self.lecture = observer(&mut commande_navigation(sonde, self.address)).await?;
                return Ok(true);
            }
        }
        for (debut, fin, mesure) in MESURES {
            if let Some(cle) = texte.strip_suffix(fin).and_then(|s| parameter(s, debut)) {
                let valeur = self
                    .lecture
                    .get(mesure)
                    .ok_or_else(|| Problem::Step("Observation du navigateur absente.".into()))?;
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
    async fn exige_chaque_mesure_d_un_depart() {
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
        let autre = world.depart_step("autre étape", &attendu("8")).await;
        assert!(!autre.unwrap());
        // Sans base de recette ni boîte rattachée, aucun des deux gestes d’archivage n’aboutit.
        for geste in [BOITE, GESTE] {
            assert!(world.depart_step(geste, &attendu("8")).await.is_err());
        }
    }
}
