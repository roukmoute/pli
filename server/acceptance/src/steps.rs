use std::{fmt::Display, net::SocketAddr, time::Duration};

use reqwest::{Client, StatusCode};
use serde_json::Value;

use crate::{Error, Example};

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
        })
    }

    fn response(&self) -> Result<&(StatusCode, String), Problem> {
        self.response
            .as_ref()
            .ok_or_else(|| Problem::Step("Aucune réponse HTTP à vérifier".into()))
    }

    pub async fn step(&mut self, text: &str, example: &Example) -> Result<(), Problem> {
        if text == "je consulte la disponibilité du webmail" {
            let url = format!("http://{}/api/v1/health", self.address);
            let response = self.client.get(url).send().await.map_err(Problem::infra)?;
            self.response = Some((
                response.status(),
                response.text().await.map_err(Problem::infra)?,
            ));
        } else if let Some(key) = parameter(text, "le service répond avec le code ") {
            let value = example_value(example, key)?;
            let code = StatusCode::from_bytes(value.as_bytes())
                .map_err(|_| Problem::Step(format!("Code HTTP invalide : {value}")))?;
            expect(self.response()?.0, code)?;
        } else if let Some(key) = parameter(text, "l’état annoncé est ") {
            let body: Value = serde_json::from_str(&self.response()?.1)
                .map_err(|e| Problem::Step(format!("Réponse JSON invalide : {e}")))?;
            let state = body["status"]
                .as_str()
                .ok_or_else(|| Problem::Step("La réponse ne contient pas d’état textuel".into()))?;
            expect(state, example_value(example, key)?)?;
        } else {
            return Err(Problem::Step("Étape sans implémentation".into()));
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
