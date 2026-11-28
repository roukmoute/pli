use super::*;

fn fils(page: &Value) -> usize {
    page["nouveaux"].as_array().unwrap().len()
}

#[tokio::test]
async fn la_taille_de_page_reglee_sert_les_requetes_sans_taille() -> Result<(), Error> {
    let app = boite()?;
    {
        let db = app.db.writer()?;
        for uid in 1..=7 {
            lettre(
                &db,
                1,
                uid,
                &format!("fil-{uid}"),
                "noa@recette.test",
                json!([]),
            )?;
        }
    }
    let reglee = router(PageConfig::parse(Some("3"))?).with_state(app.db.clone());
    let chemin = "/v1/views/reception";
    let premiere = lire(&reglee, chemin).await?;
    assert_eq!(fils(&premiere), 3);
    let suite = premiere["curseur_suivant"].as_str().unwrap();
    let deuxieme = lire(&reglee, &format!("{chemin}?curseur={suite}")).await?;
    assert_eq!(fils(&deuxieme), 3);
    assert_eq!(deuxieme["nouveaux"][0]["id"], "fil-4");
    let dossier = lire(&reglee, "/v1/folders/1/messages").await?;
    assert_eq!(fils(&dossier), 3);
    // Une taille demandée l’emporte sur le réglage, qui laisse l’import intact.
    assert_eq!(
        fils(&lire(&reglee, &format!("{chemin}?taille=5")).await?),
        5
    );
    assert_eq!(fils(&lire(&app.router(), RECEPTION).await?), 7);
    Ok(())
}

#[test]
fn le_reglage_accepte_de_1_a_100_fils_et_vaut_20_par_defaut() {
    for (texte, taille) in [
        (None, 20),
        (Some(""), 20),
        (Some("1"), 1),
        (Some("100"), 100),
    ] {
        assert_eq!(PageConfig::parse(texte).unwrap().size, taille, "{texte:?}");
    }
    for texte in ["0", "101", "-3", "vingt", " 20"] {
        let erreur = PageConfig::parse(Some(texte)).err().unwrap().to_string();
        assert_eq!(erreur, "WEBMAIL_PAGE_SIZE doit être un entier de 1 à 100.");
    }
}
