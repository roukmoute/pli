mod folders;
mod headers;

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, header::CACHE_CONTROL},
};
use ring::aead::{Aad, Nonce};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
use tower::ServiceExt;

use super::*;
use crate::test_support::{TestApp, tls_acceptor};

const ALICE: &str = "alice@recette.test";
const PASS: &str = "secret-de-recette";
const URL: &str = "/api/v1/session";

fn config() -> SessionConfig {
    SessionConfig::parse(Some(&hex::encode([42; 32])), "300").unwrap()
}

fn routes(app: &TestApp, allow_test: bool, config: SessionConfig) -> Router {
    crate::http::router(
        app.db.clone(),
        crate::AdminConfig::new(None, allow_test),
        config,
    )
    .unwrap()
}

fn add_profile(app: &TestApp, port: u16, active: bool) -> Result<(), Error> {
    app.db.writer()?.execute(
        "INSERT INTO server_profiles(name,imap_host,imap_port,imap_tls,smtp_host,smtp_port,smtp_tls,domains,active,local_test)
         VALUES ('Banc','127.0.0.1',?1,'implicit','127.0.0.1',3465,'implicit','[\"recette.test\"]',?2,1)",
        rusqlite::params![port, active],
    )?;
    Ok(())
}

struct Reply {
    status: StatusCode,
    body: Value,
    cookie: Option<String>,
}

async fn request(
    router: &Router,
    method: Method,
    url: &str,
    cookie: Option<&str>,
    json: Value,
) -> Reply {
    let mut request = Request::builder()
        .uri(url)
        .method(method)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        request = request.header(COOKIE, cookie);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(json.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    let status = response.status();
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .map(|h| h.to_str().unwrap().to_owned());
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    Reply {
        status,
        body,
        cookie,
    }
}

fn assert_error(reply: Reply, status: StatusCode, code: &str) {
    assert_eq!(reply.status, status);
    assert_eq!(reply.body["code"], code);
    assert!(reply.cookie.is_none());
}

fn credentials(address: &str) -> Value {
    json!({"address":address, "password": PASS})
}

fn cookie_pair(reply: &Reply) -> &str {
    let (pair, attributes) = reply.cookie.as_deref().unwrap().split_once(';').unwrap();
    assert_eq!(
        attributes,
        " Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=300"
    );
    pair
}

type Dialogue = Vec<(String, String)>;

fn imap_peer(listener: TcpListener, dialogs: Vec<Dialogue>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        for dialog in dialogs {
            let socket = listener.accept().await.unwrap().0;
            dialogue(socket, dialog).await;
        }
    })
}

async fn dialogue(socket: tokio::net::TcpStream, dialog: Dialogue) {
    let mut io = BufReader::new(tls_acceptor().accept(socket).await.unwrap());
    io.write_all(b"* OK recette\r\n").await.unwrap();
    for (command, reply) in dialog {
        let mut line = String::new();
        io.read_line(&mut line).await.unwrap();
        let (tag, actual) = line.trim_end().split_once(' ').unwrap();
        assert_eq!(actual, command);
        let reply = if let Some(tagged) = reply.strip_prefix("* BYE fin\r\n") {
            io.write_all(b"* BYE fin\r\n").await.unwrap();
            io.flush().await.unwrap();
            // Le client doit attendre le OK étiqueté après le BYE (RFC 9051 §3.4).
            let closed =
                tokio::time::timeout(std::time::Duration::from_millis(25), io.fill_buf()).await;
            assert!(
                closed.is_err(),
                "Connexion fermée avant la réponse à LOGOUT"
            );
            tagged
        } else {
            &reply
        };
        io.write_all(reply.replace("{tag}", tag).as_bytes())
            .await
            .unwrap();
        io.flush().await.unwrap();
    }
}

fn assert_encrypted_secret(db: &rusqlite::Connection) -> Result<(), Error> {
    let encrypted: Vec<u8> =
        db.query_row("SELECT encrypted_secret FROM accounts", [], |r| r.get(0))?;
    let (nonce, ciphertext) = encrypted.split_at(12);
    let key = config().key.unwrap();
    let decrypt = |address: &str, buf: &mut [u8]| {
        key.open_in_place(
            Nonce::try_assume_unique_for_key(nonce).unwrap(),
            Aad::from(format!("1:{address}")),
            buf,
        )
        .map(|plain| plain.to_vec())
    };
    assert!(decrypt("bob@recette.test", &mut ciphertext.to_vec()).is_err());
    assert_eq!(
        decrypt(ALICE, &mut ciphertext.to_vec()).unwrap(),
        PASS.as_bytes()
    );
    Ok(())
}

#[tokio::test]
async fn login_reuses_the_account_rotates_its_token_and_authenticates_the_secret()
-> Result<(), Error> {
    let app = TestApp::new()?;
    let router = routes(&app, true, config());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    add_profile(&app, listener.local_addr()?.port(), true)?;
    let peer = imap_peer(
        listener,
        vec![
            folders::login_dialog("NO [UNAVAILABLE] backend offline"),
            folders::login_dialog("NO denied"),
            folders::inbox_dialog(7, 1),
            folders::inbox_dialog(7, 2),
        ],
    );
    let login = |previous| {
        request(
            &router,
            Method::POST,
            URL,
            previous,
            credentials(" alice@RECETTE.TEST "),
        )
    };
    assert_error(
        login(None).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "service_indisponible",
    );
    assert_error(
        login(None).await,
        StatusCode::UNAUTHORIZED,
        "identifiants_invalides",
    );
    assert_eq!(
        app.db
            .reader()?
            .query_row("SELECT count(*) FROM users", [], |r| r.get::<_, i64>(0))?,
        0
    );
    let first = login(None).await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.body["address"], ALICE);
    let second = login(Some(cookie_pair(&first))).await;
    peer.await?;
    assert_eq!(first.body["account_id"], second.body["account_id"]);
    assert_ne!(cookie_pair(&first), cookie_pair(&second));
    let get = |cookie| request(&router, Method::GET, URL, Some(cookie), Value::Null);
    assert_error(
        get(cookie_pair(&first)).await,
        StatusCode::UNAUTHORIZED,
        "session_expiree",
    );
    let current = get(cookie_pair(&second)).await;
    assert_eq!(current.status, StatusCode::OK);
    assert_eq!(current.body, second.body);
    let db = app.db.reader()?;
    let (users, accounts, sessions): (i64, i64, i64) = db.query_row(
        "SELECT (SELECT count(*) FROM users) AS users, (SELECT count(*) FROM accounts) AS accounts,
                (SELECT count(*) FROM sessions) AS sessions",
        [],
        |r| Ok((r.get("users")?, r.get("accounts")?, r.get("sessions")?)),
    )?;
    assert_eq!((users, accounts, sessions), (1, 1, 1));
    let extra = app
        .db
        .writer()?
        .execute(
            "INSERT INTO accounts(user_id, address, server_profile_id, encrypted_secret)
         SELECT user_id, 'autre@recette.test', server_profile_id, encrypted_secret FROM accounts",
            [],
        )
        .unwrap_err();
    assert_eq!(
        extra.sqlite_error_code(),
        Some(rusqlite::ErrorCode::ConstraintViolation)
    );
    assert_encrypted_secret(&db)?;
    let (now, expires): (i64, i64) =
        db.query_row("SELECT unixepoch(), expires_at FROM sessions", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    assert!((298..=300).contains(&(expires - now)));
    assert_eq!(current.body["expires_at"], expires);
    app.db
        .writer()?
        .execute("UPDATE sessions SET expires_at=unixepoch()", [])?;
    assert_error(
        get(cookie_pair(&second)).await,
        StatusCode::UNAUTHORIZED,
        "session_expiree",
    );
    let closed = request(
        &router,
        Method::DELETE,
        URL,
        Some(cookie_pair(&second)),
        Value::Null,
    )
    .await;
    assert_eq!(closed.status, StatusCode::NO_CONTENT);
    assert!(closed.cookie.unwrap().ends_with("Max-Age=0"));
    assert_eq!(
        db.query_row("SELECT count(*) FROM sessions", [], |r| r.get::<_, i64>(0))?,
        0
    );
    Ok(())
}

#[tokio::test]
async fn credentials_only_go_to_one_enabled_matching_profile() -> Result<(), Error> {
    let app = TestApp::new()?;
    let router = routes(&app, true, config());
    let login = |body| request(&router, Method::POST, URL, None, body);
    for body in [
        Value::Null,
        credentials(""),
        credentials("@recette.test"),
        credentials("alice@"),
        credentials("a@b@recette.test"),
        credentials("a b@recette.test"),
        json!({"address":ALICE,"password":""}),
    ] {
        assert_error(
            login(body).await,
            StatusCode::UNAUTHORIZED,
            "identifiants_invalides",
        );
    }
    let closed = tokio::net::TcpSocket::new_v4()?;
    closed.bind("127.0.0.1:0".parse()?)?;
    let port = closed.local_addr()?.port();
    add_profile(&app, port, false)?;
    assert_error(
        login(credentials(ALICE)).await,
        StatusCode::UNAUTHORIZED,
        "identifiants_invalides",
    );
    add_profile(&app, port, true)?;
    assert_error(
        login(credentials("alice@autre.test")).await,
        StatusCode::UNAUTHORIZED,
        "identifiants_invalides",
    );
    assert_error(
        login(credentials(ALICE)).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "service_indisponible",
    );
    let listening = TcpListener::bind("127.0.0.1:0").await?;
    app.db.writer()?.execute(
        "UPDATE server_profiles SET imap_port=?1",
        [listening.local_addr()?.port()],
    )?;
    let (connection, failure) = tokio::join!(
        async {
            let (socket, _) =
                tokio::time::timeout(std::time::Duration::from_secs(2), listening.accept())
                    .await??;
            drop(socket);
            Ok::<(), Error>(())
        },
        login(credentials(ALICE)),
    );
    connection?;
    assert_error(
        failure,
        StatusCode::SERVICE_UNAVAILABLE,
        "service_indisponible",
    );
    for other in [
        routes(&app, false, config()),
        routes(&app, true, SessionConfig::parse(None, "60")?),
    ] {
        assert_error(
            request(&other, Method::POST, URL, None, credentials(ALICE)).await,
            StatusCode::SERVICE_UNAVAILABLE,
            "service_indisponible",
        );
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), listening.accept())
            .await
            .is_err()
    );
    add_profile(&app, port, true)?;
    assert!(admin::for_domain(&*app.db.reader()?, "recette.test").is_err());
    assert_error(
        login(credentials(ALICE)).await,
        StatusCode::SERVICE_UNAVAILABLE,
        "service_indisponible",
    );
    Ok(())
}

#[test]
fn mailbox_keeps_the_local_part_and_requires_one_domain() {
    assert_eq!(
        Mailbox::parse("  Alice@RECETTE.TEST ").unwrap().address,
        "Alice@recette.test"
    );
    for invalid in [
        "",
        "alice",
        "alice@",
        "@recette.test",
        "a@b@recette.test",
        "a b@recette.test",
        "a\u{1}b@recette.test",
    ] {
        assert!(Mailbox::parse(invalid).is_none());
    }
}

#[test]
fn token_parsing_and_configuration_fail_closed() {
    let key = hex::encode([1; 32]);
    assert!(SessionConfig::parse(Some("clé incorrecte"), "60").is_err());
    for ttl in ["0", "-1", "abc"] {
        assert!(SessionConfig::parse(Some(&key), ttl).is_err());
    }
    assert!(SessionConfig::parse(Some(""), "1").unwrap().key.is_none());
    let token = Token::generate().unwrap();
    let pair = format!("{COOKIE_NAME}={}", token.encode());
    for (value, expected) in [
        (format!("autre=1; {pair}"), true),
        (format!("{pair}; {pair}"), false),
        (format!("{COOKIE_NAME}=incorrect"), false),
        ("autre=oui".into(), false),
    ] {
        let mut h = HeaderMap::new();
        h.insert(COOKIE, value.parse().unwrap());
        assert_eq!(
            cookie_token(&h).map(|t| t.encode()),
            expected.then(|| token.encode())
        );
    }
}
