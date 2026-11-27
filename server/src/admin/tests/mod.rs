use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, header::CACHE_CONTROL},
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};
use tower::ServiceExt;

use super::*;
use crate::test_support::{TestApp, tls_acceptor};

const TOKEN: &str = "jeton-de-recette";
const BASE: &str = "/api/admin/v1/servers";

fn configured(app: &TestApp, token: Option<&str>, local: bool) -> Router {
    crate::http::router(
        app.db.clone(),
        AdminConfig::new(token.map(str::to_owned), local),
        crate::session::SessionConfig::parse(None, "60").unwrap(),
    )
}

async fn call(
    router: &Router,
    method: Method,
    suffix: &str,
    auth: Option<&str>,
    body: &Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("{BASE}{suffix}"))
        .header("content-type", "application/json");
    if let Some(auth) = auth {
        request = request.header(AUTHORIZATION, auth);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 128_000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn authorized(
    router: &Router,
    method: Method,
    suffix: &str,
    body: &Value,
) -> (StatusCode, Value) {
    call(
        router,
        method,
        suffix,
        Some(&format!("Bearer {TOKEN}")),
        body,
    )
    .await
}

#[tokio::test]
async fn protects_every_admin_operation_before_reading_the_body() -> Result<(), Error> {
    let app = TestApp::new()?;
    for (token, header) in [
        (None, Some("Bearer jeton-de-recette")),
        (Some(" "), Some("Bearer ")),
        (Some(TOKEN), None),
        (Some(TOKEN), Some("Basic jeton-de-recette")),
        (Some(TOKEN), Some("Bearer incorrect")),
    ] {
        let router = configured(&app, token, false);
        for (method, path) in [
            (Method::GET, ""),
            (Method::POST, ""),
            (Method::PUT, "/1"),
            (Method::POST, "/test"),
        ] {
            let (status, body) = call(&router, method, path, header, &Value::Null).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(body["code"], "unauthorized");
        }
    }
    Ok(())
}

fn candidate(profile: Value) -> Value {
    json!({"profile":profile, "credentials":{"username":"essai", "password":"pass-test"}})
}

async fn catalog(router: &Router) -> Value {
    let (status, catalog) = authorized(router, Method::GET, "", &Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    catalog
}

#[tokio::test]
async fn presets_and_local_certificate_exception_are_controlled_on_the_server() -> Result<(), Error>
{
    let app = TestApp::new()?;
    let production = configured(&app, Some(TOKEN), false);
    let initial = catalog(&production).await;
    assert_eq!(initial["servers"], json!([]));
    assert_eq!(initial["presets"].as_array().unwrap().len(), 1);
    let preset = &initial["presets"][0];
    assert_eq!(preset["active"], true);
    assert_eq!(preset["local_test"], false);
    assert_eq!(
        preset["imap"],
        json!({"host":"mail.infomaniak.com","port":993,"tls":"implicit"})
    );
    assert_eq!(
        preset["smtp"],
        json!({"host":"mail.infomaniak.com","port":587,"tls":"starttls"})
    );
    let local = configured(&app, Some(TOKEN), true);
    let bench = catalog(&local).await["presets"][1].clone();
    assert_eq!(bench["active"], true);
    assert_eq!(bench["local_test"], true);
    assert_eq!(bench["imap"]["port"], 3993);
    assert_eq!(bench["smtp"]["port"], 3465);
    let mut invalid = candidate(bench.clone());
    assert_eq!(
        authorized(&production, Method::POST, "/test", &invalid)
            .await
            .1["code"],
        "invalid_profile"
    );
    invalid["profile"]["smtp"]["host"] = json!("mail.infomaniak.com");
    assert_eq!(
        authorized(&local, Method::POST, "", &invalid).await.1["code"],
        "invalid_profile"
    );
    let mut zero = candidate(bench);
    zero["profile"]["smtp"]["port"] = json!(0);
    assert_eq!(
        authorized(&local, Method::POST, "/test", &zero).await.1["code"],
        "invalid_profile"
    );
    Ok(())
}

#[tokio::test]
async fn invalid_inputs_are_rejected_without_echoing_credentials() -> Result<(), Error> {
    let app = TestApp::new()?;
    let router = configured(&app, Some(TOKEN), false);
    let preset = catalog(&router).await["presets"][0].clone();
    for (pointer, value) in [
        ("/profile/name", json!(" ")),
        ("/profile/imap/port", json!(0)),
        ("/profile/smtp/host", json!("//example.test")),
        ("/profile/domains", json!(["a b"])),
        ("/credentials/username", json!("")),
        ("/credentials/password", json!("")),
        ("/profile/imap/tls", json!("pass-test")),
    ] {
        let mut body = candidate(preset.clone());
        *body.pointer_mut(pointer).unwrap() = value;
        let (status, result) = authorized(&router, Method::POST, "/test", &body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(result["code"], "invalid_profile");
        assert!(!result.to_string().contains("pass-test"));
    }
    Ok(())
}

// These peers assert the exact commands on either side of the TLS handshake.
async fn exchange<T: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut BufReader<T>,
    command: &str,
    reply: &[u8],
) {
    let mut line = String::new();
    stream.read_line(&mut line).await.unwrap();
    assert_eq!(line.trim_end(), command);
    stream.write_all(reply).await.unwrap();
}

async fn peer(imap: bool, upgrade: bool, accepts: bool) -> (Value, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let endpoint =
        json!({"host":"127.0.0.1","port":port,"tls":if upgrade {"starttls"} else {"implicit"}});
    let task = tokio::spawn(async move {
        let (stream, _) =
            tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
                .await
                .unwrap()
                .unwrap();
        let mut plain = BufReader::new(stream);
        let greeting: &[u8] = if imap {
            b"* OK test\r\n"
        } else {
            b"220 test ESMTP\r\n"
        };
        if upgrade {
            plain.write_all(greeting).await.unwrap();
            if imap {
                exchange(&mut plain, "A0001 STARTTLS", b"A0001 OK upgrade\r\n").await;
            } else {
                exchange(
                    &mut plain,
                    "EHLO localhost",
                    b"250-test\r\n250 STARTTLS\r\n",
                )
                .await;
                exchange(&mut plain, "STARTTLS", b"220 upgrade\r\n").await;
            }
        }
        let tls = tls_acceptor().accept(plain.into_inner()).await.unwrap();
        let mut secured = BufReader::new(tls);
        if !upgrade {
            secured.write_all(greeting).await.unwrap();
        }
        if imap {
            let response: &[u8] = if accepts {
                b"A0001 OK logged in\r\n"
            } else {
                b"A0001 NO denied\r\n"
            };
            exchange(
                &mut secured,
                "A0001 LOGIN \"essai\" \"pass-test\"",
                response,
            )
            .await;
        } else {
            exchange(
                &mut secured,
                "EHLO localhost",
                b"250-test\r\n250 AUTH PLAIN\r\n",
            )
            .await;
            let response: &[u8] = if accepts {
                b"235 authenticated\r\n"
            } else {
                b"535 denied\r\n"
            };
            exchange(
                &mut secured,
                "AUTH PLAIN AGVzc2FpAHBhc3MtdGVzdA==",
                response,
            )
            .await;
        }
    });
    (endpoint, task)
}

async fn exercise(
    router: &Router,
    method: Method,
    suffix: &str,
    preset: &Value,
    upgrade: bool,
    accepted: [bool; 2],
) -> (StatusCode, Value) {
    let (imap, imap_task) = peer(true, !upgrade, accepted[0]).await;
    let (smtp, smtp_task) = peer(false, upgrade, accepted[1]).await;
    let mut body = candidate(preset.clone());
    body["profile"]["imap"] = imap;
    body["profile"]["smtp"] = smtp;
    let result = authorized(router, method, suffix, &body).await;
    imap_task.await.unwrap();
    smtp_task.await.unwrap();
    result
}

#[tokio::test]
async fn only_two_successful_authentications_write_the_candidate() -> Result<(), Error> {
    let app = TestApp::new()?;
    let router = configured(&app, Some(TOKEN), true);
    let mut preset = catalog(&router).await["presets"][1].clone();
    let (status, report) =
        exercise(&router, Method::POST, "/test", &preset, true, [true, true]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["imap"]["authentication"], "ok");
    assert_eq!(report["smtp"]["authentication"], "ok");
    assert_eq!(catalog(&router).await["servers"], json!([]));
    let (status, result) = exercise(&router, Method::POST, "", &preset, true, [true, true]).await;
    assert_eq!(status, StatusCode::CREATED);
    let first = &result["server"];
    assert_eq!(first["id"], 1);
    preset["name"] = json!("Modifié");
    preset["active"] = json!(false);
    preset["domains"] = json!(["un.test", "deux.test"]);
    for accepted in [[false, true], [true, false]] {
        let (status, rejected) =
            exercise(&router, Method::PUT, "/1", &preset, false, accepted).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        for (i, protocol) in ["imap", "smtp"].iter().enumerate() {
            assert_eq!(rejected["diagnostic"][protocol]["tls"], "ok");
            assert_eq!(
                rejected["diagnostic"][protocol]["authentication"],
                if accepted[i] { "ok" } else { "failed" }
            );
        }
        assert_eq!(catalog(&router).await["servers"], json!([first]));
    }
    let (status, updated) =
        exercise(&router, Method::PUT, "/1", &preset, false, [true, true]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["server"]["id"], first["id"]);
    assert_eq!(updated["server"]["name"], "Modifié");
    assert_eq!(updated["server"]["domains"], preset["domains"]);
    assert_eq!(updated["server"]["active"], false);
    assert_eq!(
        exercise(&router, Method::PUT, "/999", &preset, true, [true, true])
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    drop(router);
    let reopened = app.reopen()?;
    assert_eq!(
        catalog(&configured(&reopened, Some(TOKEN), true)).await["servers"],
        json!([updated["server"]])
    );
    Ok(())
}

#[tokio::test]
async fn separates_dns_tcp_and_tls_failures_and_preserves_the_last_profile() -> Result<(), Error> {
    let app = TestApp::new()?;
    let router = configured(&app, Some(TOKEN), true);
    let presets = catalog(&router).await["presets"].clone();
    let (created, first) =
        exercise(&router, Method::POST, "", &presets[1], true, [true, true]).await;
    assert_eq!(created, StatusCode::CREATED);
    assert_eq!(first["server"]["name"], "Banc local");
    let mut bad = candidate(presets[0].clone());
    bad["profile"]["imap"]["host"] = json!("invalide.invalid");
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.bind("127.0.0.1:0".parse()?)?;
    let closed = socket.local_addr()?.port();
    bad["profile"]["smtp"] = json!({"host":"127.0.0.1","port":closed,"tls":"implicit"});
    let (status, rejected) = authorized(&router, Method::PUT, "/1", &bad).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        rejected["diagnostic"]["imap"],
        json!({"resolution":"failed","connection":"skipped","tls":"skipped","authentication":"skipped"})
    );
    assert_eq!(
        rejected["diagnostic"]["smtp"],
        json!({"resolution":"ok","connection":"failed","tls":"skipped","authentication":"skipped"})
    );
    assert_eq!(catalog(&router).await["servers"][0], first["server"]);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    bad["profile"]["imap"] =
        json!({"host":"127.0.0.1","port":listener.local_addr()?.port(),"tls":"implicit"});
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        assert!(tls_acceptor().accept(socket).await.is_err());
    });
    let (_, diagnostic) = authorized(&router, Method::POST, "/test", &bad).await;
    assert_eq!(
        diagnostic["imap"],
        json!({"resolution":"ok","connection":"ok","tls":"failed","authentication":"skipped"})
    );
    peer.await?;
    Ok(())
}
