use std::env;

use serde_json::{Value, json};
use tokio::net::TcpSocket;

use super::{Example, Problem, StatusCode, TOKEN, World, example_value, expect, parameter};

pub(super) fn bench_port(var: &str, default: u16) -> Result<u16, Problem> {
    env::var(var).map_or(Ok(default), |port| port.parse().map_err(Problem::infra))
}

fn candidate(profile: Value) -> Value {
    json!({"profile": profile, "credentials":{"username":"alice@recette.test", "password":"alice-test"}})
}

fn local_profile(catalog: &Value) -> Result<Value, Problem> {
    catalog["presets"]
        .as_array()
        .and_then(|presets| presets.iter().find(|profile| profile["local_test"] == true))
        .cloned()
        .ok_or_else(|| Problem::Step("Préréglage du banc introuvable".into()))
}

fn failed_profile(mut previous: Value, closed_port: u16) -> Value {
    previous["local_test"] = json!(false);
    previous["name"] = json!("Modification refusée");
    previous["imap"]["host"] = json!("invalide.invalid");
    previous["smtp"]["port"] = json!(closed_port);
    previous
}

impl World {
    fn servers_url(&self) -> String {
        format!("http://{}/api/admin/v1/servers", self.address)
    }

    async fn install_bench(&mut self) -> Result<(), Problem> {
        self.install_bench_at(bench_port("PLI_APS_IMAPS_PORT", 3993)?)
            .await
    }

    pub(super) async fn install_bench_at(&mut self, imap_port: u16) -> Result<(), Problem> {
        self.send(self.client.get(self.servers_url()).bearer_auth(TOKEN))
            .await?;
        let mut profile = local_profile(&self.json()?)?;
        profile["imap"]["port"] = json!(imap_port);
        profile["smtp"]["port"] = json!(bench_port("PLI_APS_SMTPS_PORT", 3465)?);
        self.send(
            self.client
                .post(self.servers_url())
                .bearer_auth(TOKEN)
                .json(&candidate(profile)),
        )
        .await?;
        expect(self.response()?.0, StatusCode::CREATED)
    }

    async fn reject_change(&mut self) -> Result<(), Problem> {
        let profile = self.json()?["server"].clone();
        let id = profile["id"]
            .as_i64()
            .ok_or_else(|| Problem::Step("Profil en service introuvable".into()))?;
        // Réserver le port sans écouter empêche sa réattribution pendant le diagnostic.
        let socket = TcpSocket::new_v4().map_err(Problem::infra)?;
        socket
            .bind("127.0.0.1:0".parse().map_err(Problem::infra)?)
            .map_err(Problem::infra)?;
        let closed = socket.local_addr().map_err(Problem::infra)?.port();
        self.send(
            self.client
                .put(format!("{}/{id}", self.servers_url()))
                .bearer_auth(TOKEN)
                .json(&candidate(failed_profile(profile, closed))),
        )
        .await
    }

    pub(super) async fn profile_step(&mut self, text: &str, row: &Example) -> Result<(), Problem> {
        match text {
            "je consulte les serveurs de messagerie" => {
                self.send(self.client.get(self.servers_url()).bearer_auth(TOKEN))
                    .await
            }
            "je consulte les serveurs sans jeton" => {
                self.send(self.client.get(self.servers_url())).await
            }
            "le profil du banc est en service" => self.install_bench().await,
            "je remplace sa réception par un hôte inconnu et sa soumission par un port fermé" => {
                self.reject_change().await
            }
            _ => self.profile_assertion(text, row),
        }
    }

    fn profile_assertion(&self, text: &str, row: &Example) -> Result<(), Problem> {
        let (prefix, key) = text
            .split_once('<')
            .and_then(|(prefix, _)| parameter(text, prefix).map(|key| (prefix, key)))
            .ok_or_else(unknown_step)?;
        let body = self.json()?;
        let value = match prefix {
            "le nombre de serveurs configurés est " => {
                body["servers"].as_array().map(|s| s.len().to_string())
            }
            "la réception proposée est " => endpoint(&body["presets"][0]["imap"]),
            "la soumission proposée est " => endpoint(&body["presets"][0]["smtp"]),
            "la résolution du serveur de réception indique " => {
                status(&body["diagnostic"]["imap"]["resolution"])
            }
            "la connexion au serveur de soumission indique " => {
                status(&body["diagnostic"]["smtp"]["connection"])
            }
            "la négociation TLS de soumission indique " => {
                status(&body["diagnostic"]["smtp"]["tls"])
            }
            "l’authentification de réception indique " => {
                status(&body["diagnostic"]["imap"]["authentication"])
            }
            "le profil en service est " => body["servers"]
                .as_array()
                .and_then(|servers| servers.iter().find(|s| s["active"] == true))
                .and_then(|s| s["name"].as_str())
                .map(str::to_owned),
            _ => return Err(unknown_step()),
        }
        .ok_or_else(|| Problem::Step("La réponse ne contient pas la valeur attendue".into()))?;
        expect(value.as_str(), example_value(row, key)?)
    }
}

fn unknown_step() -> Problem {
    Problem::Step("Étape sans implémentation".into())
}

fn endpoint(value: &Value) -> Option<String> {
    let tls = match value["tls"].as_str()? {
        "implicit" => "TLS implicite",
        "starttls" => "STARTTLS",
        _ => return None,
    };
    Some(format!(
        "{} {} {tls}",
        value["host"].as_str()?,
        value["port"].as_u64()?
    ))
}

fn status(value: &Value) -> Option<String> {
    Some(
        match value.as_str()? {
            "ok" => "réussi",
            "failed" => "échec",
            "skipped" => "non testé",
            _ => return None,
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_steps_send_the_selected_profile_and_modify_the_returned_id() {
        use axum::{
            Json, Router,
            routing::{get, put},
        };
        let catalog = json!({"presets":[{"name":"Hébergeur", "local_test":false},
            {"name":"Banc local","local_test":true,"imap":{"host":"127.0.0.1"},"smtp":{"host":"127.0.0.1"}}]});
        let api = Router::new()
            .route(
                "/api/admin/v1/servers",
                get(async move || Json(catalog)).post(async |Json(body): Json<Value>| {
                    let mut saved = body["profile"].clone();
                    assert_eq!(saved["name"], "Banc local");
                    assert!(saved["imap"]["port"].is_u64());
                    assert!(saved["smtp"]["port"].is_u64());
                    saved["id"] = json!(7);
                    (StatusCode::CREATED, Json(json!({"server":saved})))
                }),
            )
            .route(
                "/api/admin/v1/servers/7",
                put(async |Json(body): Json<Value>| {
                    let proposal = &body["profile"];
                    assert_eq!(proposal["imap"]["host"], "invalide.invalid");
                    assert_eq!(proposal["local_test"], false);
                    let port = proposal["smtp"]["port"].as_u64().unwrap();
                    assert!(port > 1023);
                    assert!(
                        tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
                            .await
                            .is_err()
                    );
                    StatusCode::UNPROCESSABLE_ENTITY
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut world = World::new(listener.local_addr().unwrap()).unwrap();
        tokio::select! {
            _ = axum::serve(listener, api) => panic!("Le service de test s’est arrêté"),
            () = async {
                world.install_bench().await.unwrap();
                world.reject_change().await.unwrap();
                assert_eq!(world.response().unwrap().0, StatusCode::UNPROCESSABLE_ENTITY);
            } => {}
        }
    }

    #[tokio::test]
    async fn the_acceptance_service_exposes_the_local_profile_for_failure_scenarios() {
        let dir = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut world = World::new(listener.local_addr().unwrap()).unwrap();
        tokio::select! {
            _ = webmail_server::serve_with_admin(listener, dir.path(), crate::steps::admin_config()) => panic!("Le service s’est arrêté"),
            () = async {
                world.profile_step("je consulte les serveurs de messagerie", &Example::new()).await.unwrap();
                let local = local_profile(&world.json().unwrap()).unwrap();
                assert_eq!(local["name"], "Banc local");
                assert!(local_profile(&json!({"presets":[]})).is_err());
                let failed = failed_profile(local, 42000);
                assert_eq!(failed["local_test"], false);
                assert_eq!(failed["imap"]["host"], "invalide.invalid");
                assert_eq!(failed["smtp"]["port"], 42000);
            } => {}
        }
    }

    #[test]
    fn every_assertion_checks_the_example_against_its_own_response_field() {
        let mut world = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        let body = json!({
            "servers":[{"name":"Arrêté", "active":false},{"name":"Banc", "active":true}],
            "presets":[{"imap":{"host":"lecture.test","port":993,"tls":"implicit"},
                        "smtp":{"host":"envoi.test","port":587,"tls":"starttls"}}],
            "diagnostic":{"imap":{"resolution":"ok","authentication":"failed"},
                          "smtp":{"connection":"failed","tls":"skipped"}}
        });
        world.response = Some((StatusCode::OK, body.to_string()));
        for (prefix, value) in [
            ("le nombre de serveurs configurés est ", "2"),
            (
                "la réception proposée est ",
                "lecture.test 993 TLS implicite",
            ),
            ("la soumission proposée est ", "envoi.test 587 STARTTLS"),
            ("la résolution du serveur de réception indique ", "réussi"),
            ("la connexion au serveur de soumission indique ", "échec"),
            ("la négociation TLS de soumission indique ", "non testé"),
            ("l’authentification de réception indique ", "échec"),
            ("le profil en service est ", "Banc"),
        ] {
            let text = format!("{prefix}<attendu>");
            for (expected, passes) in [(value, true), ("autre", false)] {
                let example = Example::from([("attendu".into(), expected.into())]);
                assert_eq!(world.profile_assertion(&text, &example).is_ok(), passes);
            }
        }
        assert!(status(&json!("autre")).is_none());
        assert!(endpoint(&json!({"tls":"non chiffré"})).is_none());
    }
}
