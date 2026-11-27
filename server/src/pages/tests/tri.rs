use super::*;

const ADRESSE: &str = "atelier@recette.test";
const ATTENTE: &str = "/api/v1/screener?details=true";
const HISTORIQUE: &str = "/api/v1/screener/historique";
const MESSAGES: &str = "/api/v1/screener/messages?adresse=atelier%40recette.test";

fn initialiser(db: &Connection) -> Result<(), Error> {
    db.execute(
        "INSERT INTO sender_decisions(account_id,address) VALUES(1,?1),(2,?1)",
        [ADRESSE],
    )?;
    db.execute("INSERT INTO folders(id,account_id,remote_path,role,uidvalidity,uidnext) VALUES(4,1,'Indésirables','junk',1,1)",[])?;
    lettre(db, 1, 1, "ancien", ADRESSE, json!([r"\Seen"]))?;
    lettre(
        db,
        1,
        10,
        "nouveau",
        ADRESSE,
        json!([r"\Flagged", r"\Answered", "$Forwarded"]),
    )?;
    db.execute("UPDATE messages SET has_attachments=1 WHERE uid=10", [])?;
    lettre(db, 4, 2, "indésirable", ADRESSE, json!([]))?;
    lettre(db, 3, 55, "autre-compte", ADRESSE, json!([]))?;
    Ok(())
}

fn choisir(etat: &str, destination: &str, revision: i64) -> Value {
    json!({"adresse":"atelier@RECETTE.TEST","decision":etat,"destination":destination,"revision_attendue":revision})
}

async fn poster(
    router: &Router,
    compte: Option<u8>,
    corps: Value,
) -> Result<(StatusCode, Value), Error> {
    requete(
        router,
        Request::post("/api/v1/screener/decisions").header("Content-Type", "application/json"),
        compte,
        Body::from(corps.to_string()),
    )
    .await
}

#[tokio::test]
async fn classe_ancien_et_nouveau_courrier_puis_retablit_un_refus_apres_redemarrage()
-> Result<(), Error> {
    let app = boite()?;
    initialiser(&*app.db.writer()?)?;
    let route = app.router();
    let attente = lire(&route, ATTENTE).await?;
    assert_eq!(attente["compte"], 1);
    let entree = &attente["expediteurs"][0];
    assert_eq!(entree["adresse"], ADRESSE);
    assert_eq!(entree["etat"], "en_attente");
    assert_eq!(entree["revision"], 1);
    assert!(entree["date_decision"].is_null());
    let lettres = entree["messages"].as_array().unwrap();
    assert_eq!(lettres.len(), 2);
    assert_eq!(
        lettres[0],
        json!({"uid":10,"generation":1,"fil":"nouveau","nom":"Noa","objet":"Lettre 10",
        "extrait":"Extrait 10","date":100,"non_lu":true,"etoile":true,"piece_jointe":true,"repondu":true,"transfere":true})
    );
    assert_eq!(lettres[1]["non_lu"], false);
    assert_eq!(lettres[1]["fil"], "ancien");
    assert!(entree["suivant"].is_null());
    assert_eq!(entree["apercus"].as_array().unwrap().len(), 2);
    assert_eq!(entree["apercus"][0]["id"], "nouveau");
    assert_eq!(entree["apercus"][0]["expediteur"]["adresse"], ADRESSE);
    assert_eq!(entree["apercus"][1]["non_lu"], false);
    assert_eq!(lire(&route, HISTORIQUE).await?["expediteurs"], json!([]));
    let mut revision_compte = attente["revision"].as_i64().unwrap();
    for (numero, destination) in ["flux", "justificatifs", "reception"]
        .into_iter()
        .enumerate()
    {
        let mut cmd = choisir("accepte", destination, numero as i64 + 1);
        if destination == "reception" {
            cmd.as_object_mut().unwrap().remove("destination");
        }
        let (status, choix) = poster(&route, Some(1), cmd).await?;
        assert_eq!(status, StatusCode::OK, "{choix}");
        assert_eq!(choix["decision"]["revision"], numero + 2);
        assert_eq!(choix["decision"]["destination"], destination);
        let revision = choix["revision_compte"].as_i64().unwrap();
        assert!(revision > revision_compte);
        revision_compte = revision;
        let page = lire(&route, &format!("/api/v1/views/{destination}")).await?;
        assert_eq!(page["revision"], revision);
        assert_eq!(page["nouveaux"][0]["id"], "nouveau");
        assert_eq!(page["deja_vus"][0]["id"], "ancien");
        assert_eq!(page["expediteurs_en_attente"], 0);
        for autre in ["reception", "flux", "justificatifs"] {
            if autre != destination {
                assert_eq!(page["compteurs"][autre], 0);
            }
        }
    }
    let (_, refus) = poster(&route, Some(1), choisir("refuse", "flux", 4)).await?;
    assert_eq!(refus["decision"]["destination"], "reception");
    assert!(refus["decision"]["date_decision"].is_i64());
    assert_eq!(
        lire(&route, RECEPTION).await?["compteurs"],
        json!({"reception":0,"flux":0,"justificatifs":0})
    );
    assert_eq!(
        lire(&route, "/api/v1/folders/4/messages").await?["nouveaux"][0]["id"],
        "indésirable"
    );
    assert_eq!(
        reponse(&route, ATTENTE, Some(2)).await?.1["expediteurs"][0]["messages"][0]["uid"],
        55
    );
    drop(route);
    let app = app.reopen()?;
    let route = app.router();
    lettre(&*app.db.writer()?, 1, 11, "suivante", ADRESSE, json!([]))?;
    let historique = lire(&route, HISTORIQUE).await?;
    let d = &historique["expediteurs"][0];
    assert_eq!(d["etat"], "refuse");
    assert_eq!(d["revision"], 5);
    assert_eq!(d["messages"].as_array().unwrap().len(), 3);
    assert_eq!(lire(&route, RECEPTION).await?["compteurs"]["reception"], 0);
    let (_, retour) = poster(&route, Some(1), choisir("accepte", "flux", 5)).await?;
    assert_eq!(retour["decision"]["revision"], 6);
    assert_eq!(
        lire(&route, "/api/v1/views/flux").await?["compteurs"]["flux"],
        3
    );
    lettre(
        &*app.db.writer()?,
        1,
        12,
        "suivante-admise",
        ADRESSE,
        json!([]),
    )?;
    assert_eq!(
        lire(&route, "/api/v1/views/flux").await?["compteurs"]["flux"],
        4
    );
    Ok(())
}

#[tokio::test]
async fn arbitre_deux_onglets_sur_la_revision_de_l_adresse_et_protege_chaque_compte()
-> Result<(), Error> {
    let app = boite()?;
    initialiser(&*app.db.writer()?)?;
    let route = app.router();
    let (a, b) = tokio::join!(
        poster(&route, Some(1), choisir("accepte", "flux", 1)),
        poster(&route, Some(1), choisir("refuse", "reception", 1))
    );
    let (a, b) = (a?, b?);
    let (ok, conflit) = if a.0 == StatusCode::OK {
        (a, b)
    } else {
        (b, a)
    };
    assert_eq!(ok.0, StatusCode::OK);
    assert_eq!(conflit.0, StatusCode::CONFLICT);
    assert_eq!(ok.1, conflit.1);
    assert_eq!(ok.1["decision"]["revision"], 2);
    assert_eq!(ok.1["decision"]["adresse"], ADRESSE);
    let compte2 = reponse(&route, ATTENTE, Some(2)).await?.1;
    assert_eq!(compte2["compte"], 2);
    assert_eq!(compte2["expediteurs"][0]["revision"], 1);
    let mut inconnue = choisir("accepte", "flux", 1);
    inconnue["adresse"] = json!("inconnu@recette.test");
    assert_eq!(
        poster(&route, Some(2), inconnue).await?.0,
        StatusCode::NOT_FOUND
    );
    for p in [ATTENTE, HISTORIQUE, MESSAGES] {
        assert_eq!(reponse(&route, p, None).await?.0, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(
        poster(&route, None, choisir("accepte", "flux", 1)).await?.0,
        StatusCode::UNAUTHORIZED
    );
    for mauvaise in [
        choisir("accepte", "flux", 0),
        choisir("autre", "flux", 1),
        choisir("accepte", "indesirables", 1),
        json!({}),
    ] {
        assert_eq!(
            poster(&route, Some(1), mauvaise).await?.0,
            StatusCode::BAD_REQUEST
        );
    }
    for invalide in ["abc", "@domaine", "nom@", "nom\n@recette.test"] {
        let mut cmd = choisir("refuse", "reception", 2);
        cmd["adresse"] = json!(invalide);
        assert_eq!(
            poster(&route, Some(1), cmd).await?.0,
            StatusCode::BAD_REQUEST
        );
    }
    for suffixe in ["&avant=3", "&generation=1"] {
        assert_eq!(
            reponse(&route, &format!("{MESSAGES}{suffixe}"), Some(1))
                .await?
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        reponse(
            &route,
            "/api/v1/screener/messages?adresse=absent%40recette.test",
            Some(1)
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    Ok(())
}

#[tokio::test]
async fn pagine_les_decisions_et_les_lettres_sans_melanger_deux_generations() -> Result<(), Error> {
    let app = boite()?;
    {
        let db = app.db.writer()?;
        db.execute(
            "INSERT INTO sender_decisions(account_id,address,state) VALUES(1,?1,'accepte')",
            [ADRESSE],
        )?;
        for uid in 1..=40 {
            lettre(&db, 1, uid, &format!("fil-{uid}"), ADRESSE, json!([]))?;
        }
        for n in 0..20 {
            db.execute(
                "INSERT INTO sender_decisions(account_id,address,state) VALUES(1,?1,'refuse')",
                [format!("z{n:02}@recette.test")],
            )?;
        }
    }
    let route = app.router();
    let liste = lire(&route, HISTORIQUE).await?;
    assert_eq!(liste["expediteurs"].as_array().unwrap().len(), 20);
    assert_eq!(liste["suite"], "z18@recette.test");
    let suite = lire(&route, &format!("{HISTORIQUE}?apres=z18%40recette.test")).await?;
    assert_eq!(suite["expediteurs"].as_array().unwrap().len(), 1);
    assert_eq!(suite["expediteurs"][0]["adresse"], "z19@recette.test");
    assert!(suite["suite"].is_null());
    assert!(
        lire(
            &route,
            &format!("{HISTORIQUE}?apres=atelier%40recette.test")
        )
        .await?["suite"]
            .is_null()
    );
    let premiere = lire(&route, MESSAGES).await?;
    assert_eq!(premiere["messages"].as_array().unwrap().len(), 20);
    assert_eq!(premiere["suivant"], json!({"generation":1,"uid":21}));
    let suite = lire(&route, &format!("{MESSAGES}&avant=21&generation=1")).await?;
    assert_eq!(suite["messages"].as_array().unwrap().len(), 20);
    assert_eq!(suite["messages"][0]["uid"], 20);
    assert_eq!(suite["messages"][19]["uid"], 1);
    assert!(suite["suivant"].is_null());
    assert_eq!(
        lire(&route, &format!("{MESSAGES}&avant=21&generation=2")).await?["messages"],
        json!([])
    );
    Ok(())
}

#[test]
fn met_a_jour_les_listes_qui_precedent_l_installation_du_screener() -> Result<(), Error> {
    use include_dir::include_dir;
    use rusqlite_migration::Migrations;
    static DOSSIER: include_dir::Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");
    let migrations = Migrations::from_directory(&DOSSIER)?;
    let mut db = Connection::open_in_memory()?;
    migrations.to_version(&mut db, 6)?;
    db.execute_batch(BOITES_DE_TEST)?;
    db.execute("INSERT INTO sender_decisions(account_id,address,state,destination) VALUES(1,?1,'accepte','flux')",[ADRESSE])?;
    lettre(&db, 1, 1, "ancien", ADRESSE, json!([]))?;
    lettre(&db, 1, 10, "récent", ADRESSE, json!([]))?;
    let nombre = |db: &Connection, vue: &str| {
        db.query_row(
            "SELECT total FROM list_counts WHERE account_id=1 AND scope=?1",
            [vue],
            |r| r.get::<_, i64>(0),
        )
    };
    assert_eq!(nombre(&db, "reception")?, 1);
    migrations.to_latest(&mut db)?;
    assert_eq!(nombre(&db, "reception")?, 0);
    assert_eq!(nombre(&db, "flux")?, 2);
    Ok(())
}
