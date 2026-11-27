use std::{fmt::Display, net::SocketAddr, path::PathBuf, time::Duration};

use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::Value;

use crate::{Error, Example};
mod folders;
mod privacy;
mod profiles;
mod sessions;

const TOKEN: &str = "jeton-fictif-acceptation";

pub(crate) fn admin_config() -> webmail_server::AdminConfig {
    webmail_server::AdminConfig::new(Some(TOKEN.into()), true)
}

#[derive(Debug)]
pub(crate) enum Problem {
    Step(String),
    Infrastructure(Error),
}

impl Problem {
    pub fn infra(e: impl Into<Error>) -> Self {
        Self::Infrastructure(e.into())
    }
}

pub(crate) struct World {
    address: SocketAddr,
    client: Client,
    response: Option<(StatusCode, String)>,
    session: sessions::SessionSteps,
    pub(crate) restart: Option<tokio::sync::mpsc::Sender<crate::runtime::Restart>>,
    initial_marker: Option<(u32, u32)>,
    pub(crate) recordings: Option<(PathBuf, PathBuf)>,
}

impl World {
    pub fn new(address: SocketAddr) -> Result<Self, Problem> {
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(Problem::infra)?;
        Ok(Self {
            address,
            client,
            response: None,
            session: Default::default(),
            restart: None,
            initial_marker: None,
            recordings: None,
        })
    }

    fn response(&self) -> Result<&(StatusCode, String), Problem> {
        self.response
            .as_ref()
            .ok_or_else(|| Problem::Step("Aucune réponse HTTP à vérifier".into()))
    }

    async fn send(&mut self, request: RequestBuilder) -> Result<(), Problem> {
        let response = request.send().await.map_err(Problem::infra)?;
        self.session.record_cookie(&response);
        self.response = Some((
            response.status(),
            response.text().await.map_err(Problem::infra)?,
        ));
        Ok(())
    }

    fn json(&self) -> Result<Value, Problem> {
        serde_json::from_str(&self.response()?.1)
            .map_err(|e| Problem::Step(format!("Réponse JSON invalide : {e}")))
    }

    pub async fn step(&mut self, text: &str, example: &Example) -> Result<(), Problem> {
        if text == "je consulte la disponibilité du webmail" {
            let url = format!("http://{}/api/v1/health", self.address);
            self.send(self.client.get(url)).await?;
        } else if let Some(key) = parameter(text, "le service répond avec le code ") {
            let value = example_value(example, key)?;
            let code = StatusCode::from_bytes(value.as_bytes())
                .map_err(|_| Problem::Step(format!("Code HTTP invalide : {value}")))?;
            expect(self.response()?.0, code)?;
        } else if let Some(key) = parameter(text, "l’état annoncé est ") {
            let body = self.json()?;
            let state = body["status"]
                .as_str()
                .ok_or_else(|| Problem::Step("La réponse ne contient pas d’état textuel".into()))?;
            expect(state, example_value(example, key)?)?;
        } else if !self.privacy_step(text, example)?
            && !self.session_step(text, example).await?
            && !self.folder_step(text, example).await?
        {
            self.profile_step(text, example).await?;
        }
        Ok(())
    }
}

fn parameter<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let key = text
        .strip_prefix(prefix)?
        .strip_prefix('<')?
        .strip_suffix('>')?;
    (!key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).then_some(key)
}

fn example_value<'a>(example: &'a Example, key: &str) -> Result<&'a str, Problem> {
    example
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| Problem::Step(format!("Valeur d’exemple manquante : <{key}>")))
}

fn expect<T: Display + PartialEq>(actual: T, expected: T) -> Result<(), Problem> {
    if actual == expected {
        Ok(())
    } else {
        Err(Problem::Step(format!(
            "Attendu « {expected} », reçu « {actual} »"
        )))
    }
}
