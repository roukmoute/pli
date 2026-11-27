use super::*;
use folders::{list_dialog, login_dialog, ok, status};
use std::time::Duration;

const ENDPOINT: &str = "/api/v1/screener";
const ITEMS: &str = "(UID ENVELOPE FLAGS INTERNALDATE RFC822.SIZE BODYSTRUCTURE BODY.PEEK[HEADER.FIELDS (REFERENCES)])";

fn examine(next: u32) -> (String, String) {
    ok(
        r#"EXAMINE "INBOX""#,
        &format!("* OK [UIDVALIDITY 13] génération\r\n* OK [UIDNEXT {next}] prochain\r\n"),
    )
}

fn header(uid: u32) -> String {
    format!(
        "* 1 FETCH (UID {uid} ENVELOPE (\"Wed, 1 Jan 2025 00:00:00 +0000\" \"Lettre {uid}\" ((\"Anne\" NIL \"a\" \"RECETTE.TEST\")) NIL NIL ((NIL NIL \"b\" \"recette.test\")) NIL NIL NIL \"<{uid}@recette.test>\") FLAGS (\\Flagged) INTERNALDATE \" 1-Jan-2025 01:00:00 +0000\" RFC822.SIZE 285 BODYSTRUCTURE (\"text\" \"plain\" NIL NIL NIL \"7bit\" 6 1) BODY[HEADER.FIELDS (REFERENCES)] {{0}}\r\n)\r\n"
    )
}

fn batch(ids: &[u32], returned: u32) -> Dialogue {
    let set = ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let unsolicited = "* 900 FETCH (UID 900 FLAGS (\\Seen))\r\n";
    let headers = format!("{unsolicited}{}{}", header(901), header(returned));
    let snippets = format!(
        "{unsolicited}* 1 FETCH (UID {returned} BODY[2] {{3}}\r\nnon)\r\n* 1 FETCH (UID {returned} BODY[1] {{6}}\r\nExamen)\r\n"
    );
    vec![
        ok(&format!("UID FETCH {set} {ITEMS}"), &headers),
        ok(
            &format!("UID FETCH {returned} (UID BODY.PEEK[1])"),
            &snippets,
        ),
    ]
}

async fn next_poll() {
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::time::resume();
}

#[tokio::test]
async fn fetches_fifty_recent_uids_first_then_polls_inbox_with_the_original_boundary()
-> Result<(), Error> {
    let app = TestApp::new()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    add_profile(&app, listener.local_addr()?.port(), true)?;
    let mut initial = list_dialog(13, 52, false, "");
    initial.push(examine(52));
    let found = (1..=51)
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    initial.push(ok(
        "UID SEARCH UID 1:51",
        &format!("* SEARCH {found} 901\r\n"),
    ));
    initial.extend(batch(&(2..=51).rev().collect::<Vec<_>>(), 51));
    initial.extend(batch(&[1], 1));
    initial.push(ok("LOGOUT", ""));
    let (ready, done) = tokio::sync::oneshot::channel();
    let (stored, notified) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        dialogue(listener.accept().await.unwrap().0, initial).await;
        ready.send(()).unwrap();
        let mut poll = login_dialog("OK welcome");
        // Certains serveurs ne donnent UIDNEXT que dans la réponse STATUS.
        poll.push(ok(r#"EXAMINE "INBOX""#, "* OK [UIDVALIDITY 13] stable\r\n"));
        poll.push(status("INBOX", 13, 53));
        poll.push(ok("UID SEARCH UID 52:52", "* SEARCH 52\r\n"));
        poll.extend(batch(&[52], 52));
        dialogue(listener.accept().await.unwrap().0, poll).await;
        stored.send(()).unwrap();
    });
    let router = routes(&app, true, config());
    assert_error(
        request(&router, Method::GET, ENDPOINT, None, Value::Null).await,
        StatusCode::UNAUTHORIZED,
        "session_expiree",
    );
    let logged = request(&router, Method::POST, URL, None, credentials(ALICE)).await;
    let cookie = cookie_pair(&logged);
    tokio::time::timeout(Duration::from_secs(3), done).await??;
    let read = || request(&router, Method::GET, ENDPOINT, Some(cookie), Value::Null);
    assert_eq!(read().await.body, json!({"senders":[]}));
    next_poll().await;
    tokio::time::timeout(Duration::from_secs(3), notified).await??;
    let response = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let response = read().await;
            if response.body["senders"] != json!([]) {
                break response;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    peer.await?;
    assert_eq!(
        response.body,
        json!({"senders":[{"name":"Anne","address":"a@recette.test","last_subject":"Lettre 52","snippet":"Examen","message_count":1,"revision":1}]})
    );
    let db = app.db.reader()?;
    assert_eq!(
        db.query_row(
            "SELECT group_concat(uid, ',' ORDER BY uid) FROM messages",
            [],
            |r| r.get::<_, String>(0)
        )?,
        "1,51,52"
    );
    let data: (String, i64, i64, u32, String, String) = db.query_row(
        "SELECT message_id,sent_at,received_at,size,flags,recipients FROM messages WHERE uid=52",
        [],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        },
    )?;
    assert_eq!(
        (data.0.as_str(), data.1, data.2, data.3),
        ("52@recette.test", 1735689600, 1735693200, 285)
    );
    assert_eq!(
        serde_json::from_str::<Value>(&data.4)?,
        json!(["\\Flagged"])
    );
    assert_eq!(
        serde_json::from_str::<Value>(&data.5)?,
        json!({"to":[{"address":"b@recette.test","name":null}],"cc":[],"bcc":[]})
    );
    Ok(())
}
