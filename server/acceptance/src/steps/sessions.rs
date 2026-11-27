use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{COOKIE, SET_COOKIE};
use serde_json::json;
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
};

use super::{Example, Problem, World, example_value, expect, parameter, profiles::bench_port};

#[derive(Default)]
pub(super) struct SessionSteps {
    cookie: Option<String>,
    closed_cookie: Option<String>,
    alice: Option<i64>,
    outage: tokio::task::JoinSet<()>,
}

impl SessionSteps {
    pub fn record_cookie(&mut self, response: &reqwest::Response) {
        if let Some(pair) = response
            .headers()
            .get(SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
        {
            self.cookie = (!pair.ends_with('=')).then(|| pair.to_owned());
        }
    }
}

fn expiry_delay(end: u64, now: Duration) -> Result<Duration, Problem> {
    let wait = Duration::from_secs(end).saturating_sub(now);
    if wait > Duration::from_secs(5) {
        return Err(Problem::infra(
            "La recette requiert une session de cinq secondes au plus.",
        ));
    }
    Ok(wait + Duration::from_millis(10))
}

impl World {
    fn session_url(&self) -> String {
        format!("http://{}/api/v1/session", self.address)
    }

    async fn open_mailbox(&mut self, name: &str, secret: &str) -> Result<(), Problem> {
        let address = format!("{name}@recette.test");
        self.send(
            self.client
                .post(self.session_url())
                .json(&json!({"address":address,"password":secret})),
        )
        .await
    }

    async fn unavailable_bench(&mut self, upstream: u16) -> Result<(), Problem> {
        // Le diagnostic enregistre le profil pendant que le relais dessert le vrai banc.
        // Ensuite le relais coupe les connexions sans libérer le port pendant l’exemple.
        let relay = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(Problem::infra)?;
        let local = relay.local_addr().map_err(Problem::infra)?.port();
        let forward = async {
            let (mut from, _) = relay.accept().await.map_err(Problem::infra)?;
            let mut to = TcpStream::connect(("127.0.0.1", upstream))
                .await
                .map_err(Problem::infra)?;
            copy_bidirectional(&mut from, &mut to)
                .await
                .map_err(Problem::infra)?;
            Ok(())
        };
        let forward = async {
            tokio::time::timeout(Duration::from_secs(10), forward)
                .await
                .map_err(Problem::infra)?
        };
        tokio::try_join!(self.install_bench_at(local), forward)?;
        self.session.outage.spawn(async move {
            while let Ok((connection, _)) = relay.accept().await {
                drop(connection);
            }
        });
        Ok(())
    }

    async fn read_session(&mut self, url: String, cookie: Option<String>) -> Result<(), Problem> {
        let cookie = cookie.ok_or_else(|| Problem::Step("Aucun cookie de recette.".into()))?;
        self.send(self.client.get(url).header(COOKIE, cookie)).await
    }

    async fn wait_for_expiry(&self) -> Result<(), Problem> {
        let end = self.json()?["expires_at"]
            .as_u64()
            .ok_or_else(|| Problem::Step("Échéance de session absente.".into()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(Problem::infra)?;
        tokio::time::sleep(expiry_delay(end, now)?).await;
        Ok(())
    }

    pub(super) async fn session_step(
        &mut self,
        text: &str,
        row: &Example,
    ) -> Result<bool, Problem> {
        match text {
            "Alice ouvre sa session" => self.open_mailbox("alice", "alice-test").await?,
            "Alice présente un mauvais secret" => {
                self.open_mailbox("alice", "secret-incorrect").await?
            }
            "Bob ouvre sa propre session" => {
                self.session.alice = self.json()?["account_id"].as_i64();
                self.open_mailbox("bob", "bob-test").await?;
            }
            "le serveur de réception vient de devenir inaccessible" => {
                self.unavailable_bench(bench_port("PLI_APS_IMAPS_PORT", 3993)?)
                    .await?
            }
            "je ferme ma session" => {
                let cookie = self
                    .session
                    .cookie
                    .clone()
                    .ok_or_else(|| Problem::Step("Session absente.".into()))?;
                self.session.closed_cookie = Some(cookie.clone());
                self.send(
                    self.client
                        .delete(self.session_url())
                        .header(COOKIE, cookie),
                )
                .await?;
            }
            "je consulte ma session" => {
                self.read_session(self.session_url(), self.session.cookie.clone())
                    .await?
            }
            "je consulte ma session avec mon ancien cookie" => {
                self.read_session(self.session_url(), self.session.closed_cookie.clone())
                    .await?
            }
            "je consulte ma session après son expiration" => {
                self.wait_for_expiry().await?;
                self.read_session(self.session_url(), self.session.cookie.clone())
                    .await?;
            }
            "Bob demande la session en indiquant le compte d’Alice" => {
                let alice = self
                    .session
                    .alice
                    .ok_or_else(|| Problem::Step("Compte d’Alice absent.".into()))?;
                self.read_session(
                    format!("{}?account_id={alice}", self.session_url()),
                    self.session.cookie.clone(),
                )
                .await?;
            }
            _ => return self.session_assertion(text, row),
        }
        Ok(true)
    }

    fn session_assertion(&self, text: &str, row: &Example) -> Result<bool, Problem> {
        let (prefix, _) = text.split_once('<').unwrap_or((text, ""));
        let value = match prefix {
            "la boîte ouverte est " => self.json()?["address"].as_str().map(str::to_owned),
            "le motif annoncé est " => self.json()?["code"].as_str().map(str::to_owned),
            "les seules informations de session sont " => self.json()?.as_object().map(|fields| {
                let mut keys: Vec<_> = fields.keys().map(String::as_str).collect();
                keys.sort_unstable();
                keys.join(",")
            }),
            _ => return Ok(false),
        }
        .ok_or_else(|| Problem::Step("Information absente de la réponse de session.".into()))?;
        let key = parameter(text, prefix)
            .ok_or_else(|| Problem::Step("Paramètre de session absent.".into()))?;
        expect(value.as_str(), example_value(row, key)?)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn expiry_step_waits_through_the_deadline_and_bounds_the_recipe_duration() {
        let now = Duration::from_secs(20);
        assert_eq!(expiry_delay(25, now).unwrap(), Duration::from_millis(5010));
        assert_eq!(expiry_delay(19, now).unwrap(), Duration::from_millis(10));
        assert!(expiry_delay(26, now).is_err());
    }

    #[tokio::test]
    async fn session_steps_send_the_right_credentials_and_reuse_the_returned_cookie() {
        use axum::{
            Json, Router,
            http::{HeaderMap, Uri},
            routing::get,
        };
        let api = Router::new().route("/api/v1/session", get(async |headers: HeaderMap, uri: Uri| {
            Json(json!({"cookie":headers[COOKIE].to_str().unwrap(), "query":uri.query(), "expires_at":1}))
        }).post(async |body: Json<serde_json::Value>| {
            let address = body["address"].as_str().unwrap();
            let password = body["password"].as_str().unwrap();
            let (cookie,id) = match (address,password) {
                ("alice@recette.test","alice-test") => ("sample=alice; Path=/",42),
                ("bob@recette.test","bob-test") => ("sample=bob; Path=/",84),
                ("alice@recette.test","secret-incorrect") => ("sample=; Path=/",0),
                _ => panic!("Identifiants fictifs inattendus"),
            };
            ([(SET_COOKIE,cookie)], Json(json!({"account_id":id, "expires_at":1})))
        }).delete(async |headers: HeaderMap| {
            assert_eq!(headers[COOKIE],"sample=bob");
            ([(SET_COOKIE,"sample=; Max-Age=0")], StatusCode::NO_CONTENT)
        }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut world = World::new(listener.local_addr().unwrap()).unwrap();
        tokio::select! {
            _ = axum::serve(listener, api) => panic!("Arrêt du serveur de recette"),
            () = async {
                let row=Example::new();
                world.step("Alice présente un mauvais secret",&row).await.unwrap();
                assert!(world.session.cookie.is_none());
                world.step("Alice ouvre sa session",&row).await.unwrap();
                world.step("je consulte ma session après son expiration",&row).await.unwrap();
                assert_eq!(world.json().unwrap()["cookie"],"sample=alice");
                world.step("Alice ouvre sa session",&row).await.unwrap();
                world.step("Bob ouvre sa propre session",&row).await.unwrap();
                world.step("Bob demande la session en indiquant le compte d’Alice",&row).await.unwrap();
                assert_eq!(world.json().unwrap()["query"],"account_id=42");
                assert_eq!(world.json().unwrap()["cookie"],"sample=bob");
                world.step("je consulte ma session",&row).await.unwrap();
                assert!(world.json().unwrap()["query"].is_null());
                world.step("je ferme ma session",&row).await.unwrap();
                assert!(world.session.cookie.is_none());
                world.step("je consulte ma session avec mon ancien cookie",&row).await.unwrap();
                assert_eq!(world.json().unwrap()["cookie"],"sample=bob");
            } => {}
        }
    }

    #[tokio::test]
    async fn outage_disconnects_the_client_after_a_successful_exchange() {
        use axum::{Json, Router, routing::get};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = upstream.local_addr().unwrap().port();
        let api = Router::new().route(
            "/api/admin/v1/servers",
            get(async || {
                Json(json!({
                    "presets":[{"local_test":true,"imap":{},"smtp":{}}]
                }))
            })
            .post(async |Json(body): Json<serde_json::Value>| {
                let port = body["profile"]["imap"]["port"].as_u64().unwrap() as u16;
                let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
                socket.write_u8(42).await.unwrap();
                assert_eq!(socket.read_u8().await.unwrap(), 84);
                (StatusCode::CREATED, Json(json!({"port":port})))
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut world = World::new(listener.local_addr().unwrap()).unwrap();
        let exchange = async {
            let (mut stream, _) = upstream.accept().await.unwrap();
            assert_eq!(stream.read_u8().await.unwrap(), 42);
            stream.write_u8(84).await.unwrap();
            Ok::<(), Problem>(())
        };
        tokio::select! {
            _ = axum::serve(listener, api) => panic!("Arrêt du serveur de recette"),
            result = async { tokio::try_join!(exchange, world.unavailable_bench(upstream_port)) } => { result.unwrap(); }
        }
        let port = world.json().unwrap()["port"].as_u64().unwrap() as u16;
        let mut disconnected = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let read = tokio::time::timeout(Duration::from_secs(2), disconnected.read_u8())
            .await
            .unwrap();
        assert_eq!(read.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn assertions_compare_the_expected_address_reason_and_fields() {
        let mut world = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        world.response = Some((
            StatusCode::OK,
            json!({"address":"a@recette.test","code":"essai"}).to_string(),
        ));
        for (field, value) in [
            ("la boîte ouverte est ", "a@recette.test"),
            ("le motif annoncé est ", "essai"),
            ("les seules informations de session sont ", "address,code"),
        ] {
            let step = format!("{field}<valeur>");
            let example = |v: &str| Example::from([("valeur".into(), v.into())]);
            assert!(world.session_assertion(&step, &example(value)).unwrap());
            assert!(
                world
                    .session_assertion(&step, &example("inattendu"))
                    .is_err()
            );
        }
        world.response = Some((StatusCode::OK, serde_json::Value::Null.to_string()));
        assert!(
            world
                .session_assertion("la boîte ouverte est <boite>", &Example::new())
                .is_err()
        );
    }
}
