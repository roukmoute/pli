use super::*;

const FOLDERS: &str = "/api/v1/folders";

pub(super) fn login_dialog(result: &str) -> Dialogue {
    vec![(
        format!("LOGIN \"{ALICE}\" \"{PASS}\""),
        format!("{{tag}} {result}\r\n"),
    )]
}

pub(super) fn ok(command: &str, responses: &str) -> (String, String) {
    (command.into(), format!("{responses}{{tag}} OK terminé\r\n"))
}

pub(super) fn status(name: &str, validity: u32, next: u32) -> (String, String) {
    ok(
        &format!("STATUS \"{name}\" (UIDVALIDITY UIDNEXT)"),
        &format!("* STATUS \"{name}\" (UIDNEXT {next} UIDVALIDITY {validity})\r\n"),
    )
}

pub(super) fn list_dialog(validity: u32, next: u32, special: bool, extra_names: &str) -> Dialogue {
    let mut dialog = login_dialog("OK welcome");
    dialog.push(status("INBOX", validity, next));
    let mut list = r#"LIST "" "*""#.to_owned();
    let mut caps = "* CAPABILITY IMAP4rev1".to_owned();
    if special {
        list.push_str(" RETURN (SPECIAL-USE)");
        caps.push_str(" special-use");
    }
    dialog.push(ok("CAPABILITY", &format!("{caps}\r\n")));
    dialog.push(ok(
        &list,
        &format!("* LIST () \"/\" INBOX\r\n{extra_names}"),
    ));
    dialog
}

fn empty_history(dialog: &mut Dialogue, path: &str, validity: u32, next: u32) {
    if next > 1 {
        dialog.push(ok(
            &format!("EXAMINE \"{path}\""),
            &format!("* OK [UIDVALIDITY {validity}] stable\r\n"),
        ));
        dialog.push(ok(
            &format!("UID SEARCH UID 1:{}", next - 1),
            "* SEARCH\r\n",
        ));
    }
}

pub(super) fn inbox_dialog(validity: u32, next: u32) -> Dialogue {
    let mut dialog = list_dialog(validity, next, false, "");
    empty_history(&mut dialog, "INBOX", validity, next);
    dialog.push(ok("LOGOUT", "* BYE fin\r\n"));
    dialog
}

fn marker(app: &TestApp) -> Result<(u32, u32), Error> {
    Ok(app
        .db
        .reader()?
        .query_row("SELECT uidvalidity, uidnext FROM inbox_markers", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?)
}

#[tokio::test]
async fn an_account_saved_before_folder_discovery_starts_without_an_open_session()
-> Result<(), Error> {
    let app = TestApp::new()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    add_profile(&app, listener.local_addr()?.port(), true)?;
    let key = config().key.unwrap();
    let encrypted = secret::encrypt(&key, 1, ALICE, PASS.into())?;
    for (profile, address) in [(2, ALICE), (1, "bob@recette.test")] {
        assert!(decrypt(&key, profile, address, &mut encrypted.clone()).is_err());
    }
    assert!(decrypt(&key, 1, ALICE, &mut [0; 5]).is_err());
    {
        let writer = app.db.writer()?;
        writer.execute("INSERT INTO users DEFAULT VALUES", [])?;
        writer.execute(
            "INSERT INTO accounts(user_id,server_profile_id,address,encrypted_secret)
                        VALUES(last_insert_rowid(),1,?1,?2)",
            rusqlite::params![ALICE, encrypted],
        )?;
    }
    let peer = imap_peer(listener, vec![inbox_dialog(59, 8)]);
    let _router = routes(&app, true, config());
    tokio::time::timeout(std::time::Duration::from_secs(5), peer).await??;
    assert_eq!(marker(&app)?, (59, 8));
    let sessions: u32 = app
        .db
        .reader()?
        .query_row("SELECT count(*) FROM sessions", [], |r| r.get(0))?;
    assert_eq!(sessions, 0);
    Ok(())
}

#[tokio::test]
async fn discovers_special_use_and_resumes_from_the_encrypted_account_after_restart()
-> Result<(), Error> {
    let app = TestApp::new()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    add_profile(&app, listener.local_addr()?.port(), true)?;
    let names = "* LIST (\\Trash) \"/\" Gesendet\r\n* LIST () NIL Sent\r\n* LIST (\\Noselect) \"/\" Classement\r\n* LIST (\\HasNoChildren) \"/\" Archives\r\n";
    let mut first = list_dialog(42, 21, true, names);
    first.extend([
        status("Gesendet", 30, 2),
        status("Sent", 11, 1),
        status("Archives", 31, 1),
    ]);
    empty_history(&mut first, "INBOX", 42, 21);
    empty_history(&mut first, "Gesendet", 30, 2);
    first.push(ok("LOGOUT", ""));
    let mut second = list_dialog(52, 101, false, "* LIST () \"/\" Envoy&AOk-s\r\n");
    second.push(status("Envoy&AOk-s", 27, 3));
    empty_history(&mut second, "INBOX", 52, 101);
    empty_history(&mut second, "Envoy&AOk-s", 27, 3);
    second.push(ok("LOGOUT", ""));
    let (imported, finished) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (first_socket, _) = listener.accept().await.unwrap();
        super::dialogue(first_socket, first).await;
        let _ = imported.send(());
        imap_peer(listener, vec![second]).await.unwrap();
    });
    let router = routes(&app, true, config());
    assert_error(
        request(&router, Method::GET, FOLDERS, None, Value::Null).await,
        StatusCode::UNAUTHORIZED,
        "session_expiree",
    );
    let opened = request(&router, Method::POST, URL, None, credentials(ALICE)).await;
    assert_eq!(opened.status, StatusCode::OK);
    let cookie = cookie_pair(&opened);
    let list = request(&router, Method::GET, FOLDERS, Some(cookie), Value::Null)
        .await
        .body;
    let folders = list["folders"].as_array().unwrap();
    assert_eq!(folders.len(), 5);
    let folder = |path: &str| folders.iter().find(|f| f["path"] == path).unwrap();
    assert_eq!(folder("Gesendet")["role"], "trash");
    assert!(folder("Sent")["role"].is_null());
    assert!(folder("Sent")["delimiter"].is_null());
    assert!(folder("Classement")["role"].is_null());
    assert_eq!(folder("Archives")["role"], "archive");
    assert_eq!(folder("INBOX")["role"], "inbox");
    let inbox_id = folder("INBOX")["id"].as_i64().unwrap();
    assert_eq!(marker(&app)?, (42, 21));
    app.db
        .writer()?
        .execute("UPDATE folders SET hidden=1 WHERE id=?1", [inbox_id])?;
    finished.await?;
    drop(router);

    let app = app.reopen()?;
    let router = routes(&app, true, config());
    tokio::time::timeout(std::time::Duration::from_secs(5), peer).await??;
    let list = request(&router, Method::GET, FOLDERS, Some(cookie), Value::Null)
        .await
        .body;
    let folders = list["folders"].as_array().unwrap();
    assert_eq!(folders.len(), 2);
    assert_eq!(folders[0]["path"], "Envoyés");
    assert_eq!(folders[0]["role"], "sent");
    let remote_path: String = app.db.reader()?.query_row(
        "SELECT remote_path FROM folders WHERE role='sent'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(remote_path, "Envoy&AOk-s");
    assert_eq!(folders[1]["id"], inbox_id);
    assert_eq!(folders[1]["hidden"], true);
    let current: (u32, u32) = app.db.reader()?.query_row(
        "SELECT uidvalidity, uidnext FROM folders WHERE id=?1",
        [inbox_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    assert_eq!(current, (52, 101));
    assert_eq!(marker(&app)?, (42, 21));
    Ok(())
}

#[tokio::test]
async fn partial_discovery_cannot_publish_a_marker_or_a_session_and_can_be_retried()
-> Result<(), Error> {
    let app = TestApp::new()?;
    let router = routes(&app, true, config());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    add_profile(&app, listener.local_addr()?.port(), true)?;
    let mut partial = list_dialog(13, 4, true, "");
    partial.last_mut().unwrap().1 = "* LIST () NIL INBOX\r\n{tag} NO incomplet\r\n".into();
    let mut missing = login_dialog("OK welcome");
    missing.push(ok(
        "STATUS \"INBOX\" (UIDVALIDITY UIDNEXT)",
        "* STATUS INBOX (UIDNEXT 4)\r\n",
    ));
    let peer = imap_peer(listener, vec![partial, missing, inbox_dialog(13, 5)]);
    for _ in 0..2 {
        assert_error(
            request(&router, Method::POST, URL, None, credentials(ALICE)).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "service_indisponible",
        );
        for table in ["folders", "inbox_markers", "sessions"] {
            let n: u32 =
                app.db
                    .reader()?
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
            assert_eq!(n, 0);
        }
    }
    let opened = request(&router, Method::POST, URL, None, credentials(ALICE)).await;
    assert_eq!(opened.status, StatusCode::OK);
    assert_eq!(marker(&app)?, (13, 5));
    peer.await?;
    Ok(())
}

#[tokio::test]
async fn writing_the_marker_and_the_folders_is_one_transaction() -> Result<(), Error> {
    let app = TestApp::new()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    add_profile(&app, listener.local_addr()?.port(), true)?;
    app.db.writer()?.execute_batch(
        "CREATE TRIGGER reject_marker BEFORE INSERT ON inbox_markers
        BEGIN SELECT RAISE(ABORT, 'écriture refusée'); END;",
    )?;
    let failed = list_dialog(77, 6, false, "");
    let peer = imap_peer(listener, vec![failed, inbox_dialog(77, 7)]);
    let router = routes(&app, true, config());
    assert_error(
        request(&router, Method::POST, URL, None, credentials(ALICE)).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "service_indisponible",
    );
    assert_eq!(
        app.db
            .reader()?
            .query_row("SELECT count(*) FROM folders", [], |r| r.get::<_, u32>(0))?,
        0
    );
    app.db
        .writer()?
        .execute_batch("DROP TRIGGER reject_marker")?;
    assert_eq!(
        request(&router, Method::POST, URL, None, credentials(ALICE))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(marker(&app)?, (77, 7));
    peer.await?;
    Ok(())
}
