use super::*;

#[tokio::test]
async fn une_vue_reste_consultable_avant_la_decouverte_des_dossiers() -> Result<(), Error> {
    let app = boite()?;
    app.db
        .writer()?
        .execute("DELETE FROM folders WHERE account_id=1", [])?;
    let page = lire(&app.router(), RECEPTION).await?;
    assert_eq!(
        page["couverture"],
        json!({"import_en_cours":true,"indexes":0,"attendus":null,"proportion_indexee":null})
    );
    assert_eq!(
        page["compteurs"],
        json!({"reception":0,"flux":0,"justificatifs":0})
    );
    Ok(())
}

#[tokio::test]
async fn seule_la_partie_admise_du_fil_contribue_aux_sections_et_aux_compteurs() -> Result<(), Error>
{
    let app = boite()?;
    let router = app.router();
    let adresse = "atelier@recette.test";
    {
        let db = app.db.writer()?;
        db.execute_batch(
            "INSERT INTO sender_decisions(account_id,address) VALUES(1,'atelier@recette.test')",
        )?;
        lettre(
            &db,
            1,
            1,
            "commun",
            "autre@recette.test",
            json!([r"\Seen", r"\Flagged", r"\Answered"]),
        )?;
        lettre(
            &db,
            1,
            2,
            "commun",
            adresse,
            json!([r"\Seen", "$Forwarded"]),
        )?;
        db.execute("UPDATE messages SET has_attachments=1 WHERE uid=1", [])?;
        lettre(&db, 1, 3, "inédit", adresse, json!([]))?;
        lettre(&db, 1, 10, "commun", adresse, json!([]))?;
        lettre(&db, 1, 11, "privé", adresse, json!([]))?;
        lettre(&db, 2, 1, "atelier", adresse, json!([]))?;
    }
    let page = lire(&router, RECEPTION).await?;
    assert_eq!(page["vue"], "reception");
    assert!(page.get("dossier").is_none());
    assert_eq!(page["compte"], 1);
    assert_eq!(page["expediteurs_en_attente"], 1);
    assert_eq!(
        page["compteurs"],
        json!({"reception":2,"flux":0,"justificatifs":0})
    );
    assert_eq!(page["nouveaux"][0]["id"], "inédit");
    assert_eq!(
        page["deja_vus"],
        json!([{"id":"commun","objet":"Lettre 2","expediteur":{"nom":"Noa","adresse":adresse},
        "date":100,"extrait":"Extrait 2","nombre_messages":2,"non_lu":false,"etoile":true,"piece_jointe":true,
        "repondu":true,"transfere":true}])
    );
    assert_eq!(
        lire(&router, "/api/v1/folders/1/messages").await?["deja_vus"],
        page["deja_vus"]
    );
    let atelier = lire(&router, "/api/v1/folders/2/messages").await?;
    assert!(atelier.get("vue").is_none());
    assert_eq!(atelier["dossier"], 2);
    assert_eq!(atelier["nouveaux"][0]["id"], "atelier");
    for destination in ["flux", "justificatifs"] {
        app.db.writer()?.execute(
            "UPDATE sender_decisions SET state='accepte',destination=?1",
            [destination],
        )?;
        let routed = lire(&router, &format!("/api/v1/views/{destination}")).await?;
        assert_eq!(routed["nouveaux"].as_array().unwrap().len(), 3);
        assert_eq!(routed["nouveaux"][0]["id"], "privé");
        assert_eq!(routed["nouveaux"][1]["id"], "inédit");
        assert_eq!(routed["nouveaux"][2]["objet"], "Lettre 10");
        assert_eq!(routed["compteurs"][destination], 3);
        assert_eq!(routed["expediteurs_en_attente"], 0);
        let restant = lire(&router, RECEPTION).await?;
        assert_eq!(restant["deja_vus"][0]["objet"], "Lettre 1");
        assert_eq!(restant["deja_vus"][0]["nombre_messages"], 1);
    }
    app.db
        .writer()?
        .execute("UPDATE sender_decisions SET state='refuse'", [])?;
    let refuse = lire(&router, RECEPTION).await?;
    assert_eq!(
        refuse["compteurs"],
        json!({"reception":1,"flux":0,"justificatifs":0})
    );
    assert_eq!(refuse["expediteurs_en_attente"], 0);
    let physique = lire(&router, "/api/v1/folders/1/messages").await?;
    assert_eq!(physique["nouveaux"], json!([]));
    assert_eq!(physique["deja_vus"], refuse["deja_vus"]);
    Ok(())
}

#[tokio::test]
async fn import_partiel_reprise_et_revision_sont_atomiques() -> Result<(), Error> {
    let app = boite()?;
    let router = app.router();
    let vide = lire(&router, "/api/v1/folders/2/messages").await?;
    assert_eq!(
        vide["couverture"],
        json!({"import_en_cours":false,"indexes":0,"attendus":0,"proportion_indexee":1.0})
    );
    let inconnu = lire(&router, RECEPTION).await?;
    assert_eq!(
        inconnu["couverture"],
        json!({"import_en_cours":true,"indexes":0,"attendus":null,"proportion_indexee":null})
    );
    let adresse = "noa@recette.test";
    {
        let mut db = app.db.writer()?;
        let tx = db.transaction()?;
        tx.execute("INSERT INTO folder_imports VALUES(1,1,10,4,0)", [])?;
        lettre(&tx, 1, 9, "neuf", adresse, json!([]))?;
        lettre(&tx, 1, 7, "sept", adresse, json!([]))?;
        tx.commit()?;
    }
    let partielle = lire(&router, RECEPTION).await?;
    assert_eq!(
        partielle["couverture"],
        json!({"import_en_cours":true,"indexes":2,"attendus":4,"proportion_indexee":0.5})
    );
    assert_eq!(partielle["nouveaux"].as_array().unwrap().len(), 2);
    assert!(partielle["revision"].as_i64().unwrap() > inconnu["revision"].as_i64().unwrap());
    {
        let mut db = app.db.writer()?;
        let tx = db.transaction()?;
        lettre(&tx, 1, 5, "cinq", adresse, json!([]))?;
        // Une deuxième connexion lit la dernière révision validée pendant une écriture.
        let mut lecteur = app.db.reader()?;
        let visible =
            storage::read(&mut lecteur, 1, Scope::View("reception".into()), 20, None)?.unwrap();
        assert_eq!(serde_json::to_value(visible)?, partielle);
        tx.rollback()?;
    }
    drop(router);
    let app = app.reopen()?;
    let router = app.router();
    assert_eq!(lire(&router, RECEPTION).await?, partielle);
    {
        let mut db = app.db.writer()?;
        let tx = db.transaction()?;
        assert_eq!(lettre(&tx, 1, 9, "neuf", adresse, json!([]))?, 0);
        assert_eq!(lettre(&tx, 1, 7, "sept", adresse, json!([]))?, 0);
        tx.commit()?;
    }
    assert_eq!(lire(&router, RECEPTION).await?, partielle);
    {
        let mut db = app.db.writer()?;
        let tx = db.transaction()?;
        lettre(&tx, 1, 5, "cinq", adresse, json!([]))?;
        lettre(&tx, 1, 2, "deux", adresse, json!([]))?;
        tx.commit()?;
    }
    let fini = lire(&router, RECEPTION).await?;
    assert_eq!(
        fini["couverture"],
        json!({"import_en_cours":false,"indexes":4,"attendus":4,"proportion_indexee":1.0})
    );
    assert_eq!(fini["compteurs"]["reception"], 4);
    assert!(fini["revision"].as_i64().unwrap() > partielle["revision"].as_i64().unwrap());
    Ok(())
}

#[test]
fn les_modifications_de_fils_actualisent_uniquement_le_compte_concerne() -> Result<(), Error> {
    let app = boite()?;
    let db = app.db.writer()?;
    let vue = |compte| -> Result<Page, Error> {
        let mut lecteur = app.db.reader()?;
        Ok(storage::read(
            &mut lecteur,
            compte,
            Scope::View("reception".into()),
            20,
            None,
        )?
        .unwrap())
    };
    let bob = serde_json::to_value(vue(2)?)?;
    let adresse = "noa@recette.test";
    lettre(&db, 1, 3, "réponse", adresse, json!([]))?;
    lettre(&db, 1, 4, "parent", adresse, json!([r"\Seen"]))?;
    db.execute(
        "UPDATE messages SET thread_id='parent',flags=?1 WHERE uid=3",
        [json!([r"\Seen"]).to_string()],
    )?;
    let fusion = vue(1)?;
    assert_eq!(fusion.compteurs.reception, 1);
    assert!(fusion.nouveaux.is_empty());
    assert_eq!(fusion.deja_vus[0].nombre_messages, 2);
    db.execute("DELETE FROM messages WHERE uid=4", [])?;
    let restant = vue(1)?;
    assert_eq!(restant.deja_vus[0].objet, "Lettre 3");
    assert_eq!(restant.deja_vus[0].nombre_messages, 1);
    assert!(restant.revision > fusion.revision);
    db.execute("DELETE FROM folders WHERE id=1", [])?;
    let retire = vue(1)?;
    assert_eq!(retire.compteurs.reception, 0);
    assert!(retire.deja_vus.is_empty());
    assert!(retire.revision > restant.revision);
    assert_eq!(serde_json::to_value(vue(2)?)?, bob);
    Ok(())
}

#[test]
fn une_base_deja_importee_reconstruit_les_listes_et_utilise_l_index_de_page() -> Result<(), Error> {
    let mut db = Connection::open_in_memory()?;
    for migration in [
        include_str!("../../../migrations/001-identifier-la-base/up.sql"),
        include_str!("../../../migrations/002-profils-serveur/up.sql"),
        include_str!("../../../migrations/003-sessions-courrier/up.sql"),
        include_str!("../../../migrations/004-dossiers-et-repere/up.sql"),
        include_str!("../../../migrations/005-import-et-tri/up.sql"),
    ] {
        db.execute_batch(migration)?;
    }
    db.execute_batch(BOITES_DE_TEST)?;
    lettre(&db, 1, 3, "historique", "noa@recette.test", json!([]))?;
    db.execute_batch(include_str!(
        "../../../migrations/006-pages-de-courrier/up.sql"
    ))?;
    let page = storage::read(&mut db, 1, Scope::View("reception".into()), 20, None)?.unwrap();
    assert_eq!(page.nouveaux[0].id, "historique");
    assert_eq!(page.compteurs.reception, 1);
    assert_eq!(page.revision, 0);
    let sql = format!(
        "EXPLAIN QUERY PLAN {} AND (p.received_at,p.item)<(?4,?5) ORDER BY p.received_at DESC,p.item DESC LIMIT ?3",
        storage::SELECT_PAGE
    );
    let plan = db
        .prepare(&sql)?
        .query_map(
            params![page.compte, "reception", 51, 100, "historique"],
            |r| r.get::<_, String>(3),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .join("\n");
    assert!(plan.contains("SEARCH p USING INDEX list_page"), "{plan}");
    assert!(!plan.contains("TEMP B-TREE"), "{plan}");
    Ok(())
}
