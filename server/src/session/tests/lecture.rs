use super::*;
use crate::test_support::BOITES_DE_TEST;
use folders::{login_dialog, ok};

const ID: &str = "/api/v1/messages/1-1-3";
const FIL: &str = "/api/v1/threads/1%2Ffil";
const MIME: &str = concat!(
    "MIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=x\r\n\r\n",
    "--x\r\nContent-Type: text/html; charset=utf-8\r\n\r\n",
    "<blockquote>Été à 京都</blockquote><script type=text/plain>INERTE</script>",
    "<form>FORMULAIRE</form><img onerror='' src='https://images.recette.test/photo' alt='Jardin'>",
    "<img src='cid:timbre%40recette.test'><img src='cid:inconnue'>\r\n",
    "--x\r\nContent-Type: image/png\r\nContent-ID: <timbre@recette.test>\r\n",
    "Content-Disposition: inline; filename=timbre.png\r\nContent-Transfer-Encoding: base64\r\n\r\nUE5H\r\n",
    "--x--\r\n"
);

fn preparer(app: &TestApp, port: u16) -> Result<String, Error> {
    let mut db = app.db.writer()?;
    db.execute_batch(BOITES_DE_TEST)?;
    let secret = secret::encrypt(&config().key.unwrap(), 1, ALICE, PASS.to_owned()).unwrap();
    db.execute(
        "UPDATE server_profiles SET domains='[\"recette.test\"]',imap_port=?1 WHERE id=1",
        [port],
    )?;
    db.execute(
        "UPDATE accounts SET encrypted_secret=?1 WHERE id=1",
        [secret],
    )?;
    db.execute_batch(r#"INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,sender_address,sender_name,subject,received_at,recipients,flags,size,has_attachments,snippet)
        VALUES(1,1,1,3,'1/fil','noa@recette.test','Noa','Le jardin',100,'{"to":[]}','["\\Flagged"]',30,1,'Jardin');"#)?;
    let token = Token::generate()?;
    store::open(&mut db, 1, ALICE.to_owned(), &token, None, 300)?;
    Ok(format!("{COOKIE_NAME}={}", token.encode()))
}

fn selection(commande: &str, generation: u32) -> Dialogue {
    let mut dialogue = login_dialog("OK bienvenue");
    dialogue.push(ok(
        &format!("{commande} \"INBOX\""),
        &format!("* OK [UIDVALIDITY {generation}] valable\r\n"),
    ));
    dialogue
}

#[tokio::test]
async fn precharge_sans_marquer_et_persiste_le_rendu_apres_un_examen() -> Result<(), Error> {
    let app = TestApp::new()?;
    let router = routes(&app, true, config());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let cookie = preparer(&app, listener.local_addr()?.port())?;
    let mut lecture = selection("EXAMINE", 1);
    lecture.push(ok("UID FETCH 3 (UID BODY.PEEK[])",&format!("* 7 FETCH (UID 8 BODY[] {{3}}\r\nnon)\r\n* 3 FETCH (UID 3 BODY[] {{{}}}\r\n{MIME})\r\n* 3 FETCH (UID 3 FLAGS ())\r\n",MIME.len())));
    let mut ouverture = selection("SELECT", 1);
    ouverture.push(ok(
        "UID STORE 3 +FLAGS (\\Seen)",
        "* 3 FETCH (UID 3 FLAGS (\\Seen \\Flagged))\r\n",
    ));
    let mut absente = selection("SELECT", 1);
    absente.push(ok(
        "UID STORE 3 +FLAGS (\\Seen)",
        "* 4 FETCH (UID 4 FLAGS (\\Seen))\r\n",
    ));
    let peer = imap_peer(
        listener,
        vec![lecture, ouverture, absente, selection("EXAMINE", 2)],
    );
    let get = |path| request(&router, Method::GET, path, Some(&cookie), Value::Null);
    let headers = get(FIL).await.body;
    assert_eq!(
        headers[0],
        json!({"id":"1-1-3","objet":"Le jardin","nom":"Noa","adresse":"noa@recette.test","destinataires":{"to":[]},"date":100,"non_lu":true})
    );
    let cached = get(ID).await;
    assert_eq!(cached.status, StatusCode::OK);
    assert_eq!(cached.body["etat"], "disponible");
    assert_eq!(cached.body["version"], 1);
    let body = &cached.body["contenu"];
    let html = body["html"].as_str().unwrap();
    assert!(
        html.contains("<blockquote>Été à 京都</blockquote>") && html.contains("alt=\"Jardin\"")
    );
    for absent in ["INERTE", "FORMULAIRE", "onerror", "https://", "inconnue"] {
        assert!(!html.contains(absent));
    }
    assert_eq!(
        body["images"][0],
        json!({"type":"distante","url":"https://images.recette.test/photo"})
    );
    let part = body["images"][1]["partie"].as_u64().unwrap();
    assert_eq!(body["images"][1]["type"], "integree");
    assert_eq!(
        body["pieces"][0],
        json!({"nom":"timbre.png","mime":"image/png","taille":3})
    );
    let path = format!("{ID}/images/{part}");
    let response = router
        .clone()
        .oneshot(
            Request::get(&path)
                .header(COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.headers()["content-type"], "image/png");
    assert_eq!(to_bytes(response.into_body(), 10).await?.as_ref(), b"PNG");
    assert_eq!(get(ID).await.body, cached.body);
    assert_eq!(get(FIL).await.body, headers);
    let open = format!("{ID}/open");
    let marquer = || request(&router, Method::POST, &open, Some(&cookie), Value::Null);
    let lu = marquer().await;
    assert_eq!(lu.status, StatusCode::OK);
    assert_eq!(lu.body["fil"], "1/fil");
    assert_eq!(
        lu.body["non_lus"],
        json!({"dossier/1":false,"reception":false})
    );
    let page = get("/api/v1/views/reception").await.body;
    assert_eq!(lu.body["revision"], page["revision"]);
    assert_eq!(page["deja_vus"][0]["etoile"], true);
    assert_eq!(get(FIL).await.body[0]["non_lu"], false);
    assert_eq!(marquer().await.status, StatusCode::NOT_FOUND);
    app.db
        .writer()?
        .execute("UPDATE message_bodies SET render_version=0", [])?;
    assert_eq!(get(ID).await.body, json!({"etat":"indisponible"}));
    assert_eq!(get(&path).await.status, StatusCode::NOT_FOUND);
    peer.await?;
    let autre = Token::generate()?;
    store::open(
        &mut *app.db.writer()?,
        2,
        "bob@recette.test".into(),
        &autre,
        None,
        300,
    )?;
    for chemin in [ID, FIL, &path] {
        let autre_cookie = format!("{COOKIE_NAME}={}", autre.encode());
        assert_eq!(
            request(
                &router,
                Method::GET,
                chemin,
                Some(&autre_cookie),
                Value::Null
            )
            .await
            .status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            request(&router, Method::GET, chemin, None, Value::Null)
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
    }
    Ok(())
}

#[tokio::test]
async fn applique_les_indicateurs_par_lot_et_isole_chaque_identifiant() -> Result<(), Error> {
    let app = TestApp::new()?;
    let router = routes(&app, true, config());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let cookie = preparer(&app, listener.local_addr()?.port())?;
    app.db.writer()?.execute_batch(r#"UPDATE inbox_markers SET uidnext=9 WHERE account_id=2;
        INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,subject,received_at,recipients,flags,size,has_attachments,snippet)
        VALUES (1,1,1,4,'1/fil','Lettre',101,'{}','[]',5,0,''),
               (1,2,1,4,'1/fil','Lettre',102,'{}','[]',5,0,''),
               (2,3,1,3,'2/prive','Lettre',100,'{}','[]',5,0,'');"#)?;
    let store = |commande: &str, reponse: &str| {
        let mut d = selection("SELECT", 1);
        d.push(ok(commande, reponse));
        d
    };
    let peer = imap_peer(
        listener,
        vec![
            store(
                "UID STORE 3,4 +FLAGS (\\Seen)",
                "* 8 FETCH (UID 8 FLAGS (\\Seen))\r\n* 4 FETCH (UID 4 FLAGS ())\r\n* 3 FETCH (UID 3 FLAGS (\\SEEN))\r\n",
            ),
            store(
                "UID STORE 3 -FLAGS (\\Seen)",
                "* 3 FETCH (UID 3 FLAGS (\\Flagged))\r\n",
            ),
            store(
                "UID STORE 3 -FLAGS (\\Flagged)",
                "* 3 FETCH (UID 3 FLAGS ())\r\n",
            ),
            store(
                "UID STORE 4 +FLAGS (\\Flagged)",
                "* 4 FETCH (UID 4 FLAGS (\\Flagged))\r\n",
            ),
            store(
                "UID STORE 4 +FLAGS (\\Flagged)",
                "* 4 FETCH (UID 4 FLAGS (\\Flagged))\r\n",
            ),
            store(
                "UID STORE 3,4 +FLAGS (\\Seen)",
                "* 3 FETCH (UID 3 FLAGS (\\Seen))\r\n",
            ),
            selection("SELECT", 9),
        ],
    );
    let path = "/api/v1/messages/flags";
    let post = |v| request(&router, Method::POST, path, Some(&cookie), v);
    let marque = |ids: Value, indicateur: &str, valeur: bool| json!({ "ids":ids, "indicateur":indicateur, "valeur":valeur });
    let etat = |id: &str, etat: &str| json!({"id":id,"etat":etat});
    let lus = |ids| marque(ids, "lu", true);
    let appel = lus(json!(["1-1-3"]));
    assert_eq!(
        request(&router, Method::POST, path, None, appel.clone())
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(post(lus(json!([]))).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        post(lus(json!(vec!["inconnu"; 101]))).await.status,
        StatusCode::BAD_REQUEST
    );
    let plein = post(lus(json!(vec!["inconnu"; 100]))).await;
    assert_eq!(plein.status, StatusCode::OK);
    assert_eq!(plein.body.as_array().unwrap().len(), 100);
    let mut inconnu = appel.clone();
    inconnu["indicateur"] = "autre".into();
    assert_eq!(post(inconnu).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let lire_flags = |compte, dossier, uid| -> Result<String, Error> {
        Ok(app.db.reader()?.query_row(
            "SELECT flags FROM messages WHERE (account_id,folder_id,uid)=(?1,?2,?3)",
            rusqlite::params![compte, dossier, uid],
            |r| r.get(0),
        )?)
    };
    let fils =
        post(json!({"vue":"reception","ids":["1/fil","2/prive"],"indicateur":"lu","valeur":true}))
            .await;
    assert_eq!(fils.status, StatusCode::OK);
    assert_eq!(
        fils.body,
        json!([etat("1/fil", "partiel"), etat("2/prive", "absent")])
    );
    assert_eq!(lire_flags(1, 1, 3)?, r#"["\\Flagged","\\Seen"]"#);
    assert_eq!(lire_flags(1, 1, 4)?, "[]");
    assert_eq!(lire_flags(1, 2, 4)?, "[]");
    let ret = post(marque(
        json!(["1-1-3", "3-1-3", "absent", "1-2-3", "1-1-3"]),
        "lu",
        false,
    ))
    .await;
    assert_eq!(
        ret.body,
        json!([
            etat("1-1-3", "confirme"),
            etat("3-1-3", "absent"),
            etat("absent", "absent"),
            etat("1-2-3", "absent"),
            etat("1-1-3", "confirme")
        ])
    );
    assert_eq!(lire_flags(1, 1, 3)?, r#"["\\Flagged"]"#);
    assert_eq!(lire_flags(2, 3, 3)?, "[]");
    assert_eq!(
        post(marque(json!(["1-1-3"]), "etoile", false)).await.body[0],
        etat("1-1-3", "confirme")
    );
    assert_eq!(lire_flags(1, 1, 3)?, "[]");
    let etoiler = || post(marque(json!(["1-1-4"]), "etoile", true));
    assert_eq!(etoiler().await.body[0], etat("1-1-4", "confirme"));
    let page = || {
        request(
            &router,
            Method::GET,
            "/api/v1/views/reception",
            Some(&cookie),
            Value::Null,
        )
    };
    let avant = page().await.body;
    assert_eq!(avant["nouveaux"][0]["etoile"], true);
    assert_eq!(etoiler().await.body[0], etat("1-1-4", "confirme"));
    assert_eq!(page().await.body, avant);
    let lot = post(lus(json!(["1-1-3", "1-1-4"]))).await;
    assert_eq!(
        lot.body,
        json!([etat("1-1-3", "confirme"), etat("1-1-4", "absent")])
    );
    assert_eq!(post(appel).await.body[0], etat("1-1-3", "non_confirme"));
    peer.await?;
    Ok(())
}
