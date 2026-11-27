use super::*;

#[tokio::test]
async fn curseurs_stables_sur_dates_identiques_et_requetes_isolees() -> Result<(), Error> {
    let app = boite()?;
    let router = app.router();
    let imap = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    {
        let db = app.db.writer()?;
        db.execute(
            "UPDATE server_profiles SET imap_port=?1",
            [imap.local_addr()?.port()],
        )?;
        db.execute(
            "UPDATE inbox_markers SET uidnext=100 WHERE account_id=1",
            [],
        )?;
        for uid in 1..=25 {
            lettre(
                &db,
                1,
                uid,
                &format!("fil-{uid:02}"),
                "noa@recette.test",
                json!([]),
            )?;
        }
    }
    let premiere = lire(&router, RECEPTION).await?;
    let suite = premiere["curseur_suivant"].as_str().unwrap();
    let derniere = lire(&router, &format!("{RECEPTION}?curseur={suite}")).await?;
    assert_eq!(premiere["nouveaux"].as_array().unwrap().len(), 20);
    assert_eq!(derniere["nouveaux"].as_array().unwrap().len(), 5);
    let ids: Vec<_> = [&premiere, &derniere]
        .into_iter()
        .flat_map(|p| p["nouveaux"].as_array().unwrap())
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    let attendus: Vec<_> = (1..=25).rev().map(|i| format!("fil-{i:02}")).collect();
    assert_eq!(ids, attendus);
    assert!(derniere["curseur_suivant"].is_null());
    let exacte = lire(&router, &format!("{RECEPTION}?taille=5&curseur={suite}")).await?;
    assert_eq!(exacte, derniere);
    for size in [1, 100] {
        let page = lire(&router, &format!("{RECEPTION}?taille={size}")).await?;
        assert_eq!(page["nouveaux"].as_array().unwrap().len(), size.min(25));
    }
    for argument in [
        "taille=0",
        "taille=101",
        "taille=-1",
        "taille=non",
        "curseur=zz",
        "curseur=7b7d",
    ] {
        assert_eq!(
            reponse(&router, &format!("{RECEPTION}?{argument}"), Some(1))
                .await?
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    for path in ["/api/v1/views/flux", "/api/v1/folders/1/messages"] {
        assert_eq!(
            reponse(&router, &format!("{path}?curseur={suite}"), Some(1))
                .await?
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        reponse(&router, &format!("{RECEPTION}?curseur={suite}"), Some(2))
            .await?
            .0,
        StatusCode::BAD_REQUEST
    );
    for path in [
        "/api/v1/views/tri",
        "/api/v1/folders/3/messages",
        "/api/v1/folders/999/messages",
    ] {
        assert_eq!(
            reponse(&router, path, Some(1)).await?.0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        reponse(&router, RECEPTION, None).await?.0,
        StatusCode::UNAUTHORIZED
    );
    let (statut, autre) = reponse(&router, RECEPTION, Some(2)).await?;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(autre["compte"], 2);
    assert_eq!(autre["compteurs"]["reception"], 0);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), imap.accept())
            .await
            .is_err()
    );
    Ok(())
}
