use std::{process::Stdio, time::Duration};

use reqwest::StatusCode;
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStdout, Command},
    time::{sleep, timeout},
};

use super::{Example, Problem, World, example_value, expect, parameter};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/screener.py");
const WAIT: Duration = Duration::from_secs(12);

pub(super) struct Bench {
    _relay: Child,
    events: Lines<BufReader<ChildStdout>>,
    address: String,
}

impl Bench {
    async fn start(command: &mut Command) -> Result<(Self, u16), Problem> {
        let mut relay = command
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(Problem::infra)?;
        let mut events = BufReader::new(relay.stdout.take().expect("sortie du relais")).lines();
        let ready = event(&mut events).await?;
        let (port, address) = ready
            .split_once(' ')
            .ok_or_else(|| Problem::infra("Relais IMAP non initialisé."))?;
        Ok((
            Self {
                _relay: relay,
                events,
                address: address.to_owned(),
            },
            port.parse().map_err(Problem::infra)?,
        ))
    }

    async fn unread(&self) -> Result<String, Problem> {
        let result = Command::new("python3")
            .arg(FIXTURE)
            .arg("--unread")
            .arg(&self.address)
            .output()
            .await
            .map_err(Problem::infra)?;
        if !result.status.success() {
            return Err(Problem::infra(
                "Consultation des indicateurs IMAP impossible.",
            ));
        }
        Ok(String::from_utf8(result.stdout)
            .map_err(Problem::infra)?
            .trim()
            .to_owned())
    }
}

async fn event(events: &mut Lines<BufReader<ChildStdout>>) -> Result<String, Problem> {
    timeout(WAIT, events.next_line())
        .await
        .map_err(|_| Problem::Step("Événement du banc non observé.".into()))?
        .map_err(Problem::infra)?
        .ok_or_else(|| Problem::infra("Relais IMAP arrêté."))
}

impl World {
    async fn attach_test_mailbox(&mut self) -> Result<(), Problem> {
        let (dir, _) = self
            .recordings
            .as_ref()
            .ok_or_else(|| Problem::infra("Base de recette absente."))?;
        let (bench, port) = Bench::start(Command::new("python3").arg(FIXTURE).arg(dir)).await?;
        self.connect_bench(bench, port).await
    }

    async fn connect_bench(&mut self, bench: Bench, port: u16) -> Result<(), Problem> {
        self.install_bench_at(port).await?;
        let (name, _) = bench
            .address
            .split_once('@')
            .ok_or_else(|| Problem::infra("Boîte du banc absente."))?;
        self.open_mailbox(name, &bench.address).await?;
        expect(self.response()?.0, StatusCode::OK)?;
        self.screener_bench = Some(bench);
        Ok(())
    }

    async fn read_screener(&mut self) -> Result<(), Problem> {
        self.read_current(format!("http://{}/api/v1/screener", self.address))
            .await
    }

    async fn wait_for_delivery(&mut self) -> Result<(), Problem> {
        let bench = self
            .screener_bench
            .as_mut()
            .ok_or_else(|| Problem::infra("Banc de tri absent."))?;
        expect(event(&mut bench.events).await?.as_str(), "livré")?;
        timeout(WAIT, async {
            loop {
                self.read_screener().await?;
                if self.json()?["senders"][0]["message_count"].as_i64() == Some(2) {
                    return Ok(());
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .map_err(|_| Problem::Step("Les deux arrivées restent invisibles pendant le lot.".into()))?
    }

    async fn resume_history(&mut self) -> Result<(), Problem> {
        self.restart_service().await?;
        timeout(WAIT, async {
            loop {
                let resumed: bool = self
                    .database()?
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM messages WHERE subject='Archive A')",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(Problem::infra)?;
                if resumed {
                    return self.read_screener().await;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .map_err(|_| Problem::Step("L’historique reste incomplet après reprise.".into()))?
    }

    pub(super) async fn screener_step(
        &mut self,
        text: &str,
        row: &Example,
    ) -> Result<bool, Problem> {
        match text {
            "une boîte contenant deux anciennes lettres est rattachée" => {
                self.attach_test_mailbox().await?
            }
            "deux nouvelles lettres du même expéditeur arrivent pendant la lecture du premier lot" => {
                self.wait_for_delivery().await?
            }
            "le service redémarre au milieu du lot et reprend l’import" => {
                self.resume_history().await?
            }
            _ => return self.screener_assertion(text, row).await,
        }
        Ok(true)
    }

    async fn screener_assertion(&self, text: &str, row: &Example) -> Result<bool, Problem> {
        let Some((prefix, _)) = text.split_once('<') else {
            return Ok(false);
        };
        let value = match prefix {
            "le nombre de lettres importées dans la boîte est " => {
                self.count_letters("SELECT count(*) FROM messages")?
            }
            "le nombre de lettres admises en Réception est " => self.count_letters(
                "SELECT count(*) FROM routed_messages WHERE destination='reception'",
            )?,
            "le nombre de lettres non lues sur le serveur de courrier est " => {
                self.screener_bench
                    .as_ref()
                    .ok_or_else(|| Problem::infra("Banc de tri absent."))?
                    .unread()
                    .await?
            }
            _ => {
                let Some(value) = sender_field(prefix, &self.json()?) else {
                    return Ok(false);
                };
                value
            }
        };
        let key = parameter(text, prefix)
            .ok_or_else(|| Problem::Step("Paramètre de tri absent.".into()))?;
        expect(value.as_str(), example_value(row, key)?)?;
        Ok(true)
    }

    fn count_letters(&self, sql: &str) -> Result<String, Problem> {
        self.database()?
            .query_row(sql, [], |r| r.get::<_, u32>(0))
            .map(|v| v.to_string())
            .map_err(Problem::infra)
    }
}

fn sender_field(prefix: &str, body: &Value) -> Option<String> {
    let senders = body["senders"].as_array()?;
    let field = match prefix {
        "le nombre d’expéditeurs à trier est " => return Some(senders.len().to_string()),
        "le nombre de lettres de cet expéditeur est " => {
            return Some(senders.first()?["message_count"].as_i64()?.to_string());
        }
        "l’adresse présentée au tri est " => "address",
        "le nom présenté au tri est " => "name",
        "le dernier objet présenté est " => "last_subject",
        "le dernier extrait présenté est " => "snippet",
        _ => return None,
    };
    senders.first()?[field].as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        http::header::SET_COOKIE,
        routing::{get, post},
    };
    use rusqlite::Connection;
    use serde_json::json;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn dropping_the_fixture_stops_its_relay() {
        let command = "printf '42 temp@recette.test\\n'; exec sleep 5";
        let (bench, _) = Bench::start(Command::new("sh").args(["-c", command]))
            .await
            .unwrap();
        let Bench {
            _relay, mut events, ..
        } = bench;
        drop(_relay);
        let eof = timeout(Duration::from_secs(1), events.next_line())
            .await
            .unwrap()
            .unwrap();
        assert!(eof.is_none());
    }

    #[tokio::test]
    async fn verifies_each_expected_value_against_the_authenticated_list_and_resumed_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(dir.path().join("webmail.db")).unwrap();
        db.execute_batch(
            "CREATE TABLE messages(subject);
             INSERT INTO messages VALUES('Archive A'),('Lettre');
             CREATE TABLE routed_messages(destination);
             INSERT INTO routed_messages VALUES('reception'),(NULL);",
        )
        .unwrap();
        let profiles = get(|| async {
            Json(json!({"presets": [{"local_test": true, "imap": {}, "smtp": {}}]}))
        })
        .post(|Json(request): Json<Value>| async move {
            assert_eq!(request["profile"]["imap"]["port"], 4234);
            StatusCode::CREATED
        });
        let session = post(|Json(request): Json<Value>| async move {
            let mailbox = "temp@recette.test";
            assert_eq!(request, json!({"address": mailbox, "password": mailbox}));
            ([(SET_COOKIE, "essai=ouvert")], Json(json!({})))
        });
        let screener = get(|headers: axum::http::HeaderMap| async move {
            assert_eq!(headers["cookie"], "essai=ouvert");
            Json(json!({"senders": [{
                "address": "une@recette.test",
                "name": "Nom",
                "last_subject": "Objet",
                "snippet": "Texte",
                "message_count": 2
            }]}))
        });
        let server = Router::new()
            .route("/api/admin/v1/servers", profiles)
            .route("/api/v1/session", session)
            .route("/api/v1/screener", screener);
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut world = World::new(address).unwrap();
        assert!(world.attach_test_mailbox().await.is_err());
        world.recordings = Some((dir.path().into(), dir.path().join("log")));
        let (sender, mut restart) = tokio::sync::mpsc::channel(1);
        world.restart = Some(sender);
        let (bench, port) = Bench::start(
            Command::new("sh").args(["-c", "printf '4234 temp@recette.test\\nlivré\\n'"]),
        )
        .await
        .unwrap();
        let exercise = async {
            world.connect_bench(bench, port).await.unwrap();
            let row = Example::new();
            let arrival = "deux nouvelles lettres du même expéditeur arrivent pendant la lecture du premier lot";
            assert!(world.screener_step(arrival, &row).await.unwrap());
            let resume = "le service redémarre au milieu du lot et reprend l’import";
            let acknowledge = async { restart.recv().await.unwrap().send(address).unwrap() };
            let ((), result) = tokio::join!(acknowledge, world.screener_step(resume, &row));
            assert!(result.unwrap());
            for (prefix, expected) in [
                ("le nombre d’expéditeurs à trier est ", "1"),
                ("le nombre de lettres de cet expéditeur est ", "2"),
                ("l’adresse présentée au tri est ", "une@recette.test"),
                ("le nom présenté au tri est ", "Nom"),
                ("le dernier objet présenté est ", "Objet"),
                ("le dernier extrait présenté est ", "Texte"),
                ("le nombre de lettres importées dans la boîte est ", "2"),
                ("le nombre de lettres admises en Réception est ", "1"),
            ] {
                let step = format!("{prefix}<attendu>");
                for (value, passes) in [(expected, true), ("inattendu", false)] {
                    let row = Example::from([("attendu".into(), value.into())]);
                    let result = world.screener_assertion(&step, &row).await;
                    assert_eq!(matches!(result, Ok(true)), passes, "{step}");
                }
            }
        };
        tokio::select! {
            _ = axum::serve(listener, server) => panic!("Serveur de recette arrêté"),
            () = exercise => {}
        }
    }
}
