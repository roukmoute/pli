use std::{collections::BTreeSet, time::Duration};

use serde_json::Value;
use tokio::time::{sleep, timeout};

use super::{Example, Problem, World, example_value, expect, parameter};

fn ids(page: &Value) -> Result<Vec<String>, Problem> {
    let mut ids = Vec::new();
    for section in ["nouveaux", "deja_vus"] {
        let lignes = page[section]
            .as_array()
            .ok_or_else(|| Problem::Step(format!("Section {section} absente.")))?;
        for ligne in lignes {
            let id = ligne["id"]
                .as_str()
                .ok_or_else(|| Problem::Step("Fil sans identifiant.".into()))?;
            ids.push(id.to_string());
        }
    }
    Ok(ids)
}

impl World {
    async fn lire_pendant_import(&mut self, en_cours: bool) -> Result<(), Problem> {
        let url = format!("http://{}/api/v1/views/reception", self.address);
        timeout(Duration::from_secs(12), async {
            loop {
                self.read_current(url.clone()).await?;
                let page = self.json()?;
                if page["couverture"]["import_en_cours"].as_bool() == Some(en_cours)
                    && !ids(&page)?.is_empty()
                {
                    return Ok(());
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .map_err(|_| Problem::Step("Le lot attendu reste indisponible en Réception.".into()))?
    }

    async fn lire_deux_pages(&mut self) -> Result<(), Problem> {
        let url = format!("http://{}/api/v1/views/reception?taille=2", self.address);
        let curseur = timeout(Duration::from_secs(12), async {
            loop {
                self.read_current(url.clone()).await?;
                let page = self.json()?;
                if let Some(curseur) = page["curseur_suivant"].as_str() {
                    self.page_precedente = ids(&page)?;
                    return Ok::<_, Problem>(curseur.to_string());
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .map_err(|_| Problem::Step("La première page de Réception reste incomplète.".into()))??;
        self.read_current(format!("{url}&curseur={curseur}")).await
    }

    pub(super) async fn page_step(&mut self, text: &str, row: &Example) -> Result<bool, Problem> {
        match text {
            "je rattache une boîte dont le second lot est suspendu" => {
                self.attach_test_mailbox(&["--pages"]).await?
            }
            "je consulte la réception pendant l’import" => {
                self.lire_pendant_import(true).await?
            }
            "le service reprend le lot suspendu" => {
                self.screener_bench
                    .as_mut()
                    .ok_or_else(|| Problem::infra("Banc de pages absent."))?
                    .attendre("lot suspendu")
                    .await?;
                self.restart_service().await?;
                self.lire_pendant_import(false).await?;
            }
            "je consulte deux pages consécutives de ma réception" => {
                self.lire_deux_pages().await?
            }
            _ => return self.verifier_page(text, row),
        }
        Ok(true)
    }

    fn verifier_page(&self, text: &str, row: &Example) -> Result<bool, Problem> {
        let Some((prefix, _)) = text.split_once('<') else {
            return Ok(false);
        };
        let (valeur, suffixe) = match prefix {
            "chaque page contient " => {
                let key = parameter(text.trim_end_matches(" fils"), prefix)
                    .ok_or_else(|| Problem::Step("Taille des pages absente.".into()))?;
                expect(
                    self.page_precedente.len().to_string().as_str(),
                    example_value(row, key)?,
                )?;
                (ids(&self.json()?)?.len().to_string(), " fils")
            }
            "les deux pages présentent " => {
                let mut distincts: BTreeSet<_> = self.page_precedente.iter().cloned().collect();
                distincts.extend(ids(&self.json()?)?);
                (distincts.len().to_string(), " fils distincts")
            }
            "la réception indique " => (
                self.json()?["expediteurs_en_attente"].to_string(),
                " expéditeurs en attente",
            ),
            "le nombre de fils en Réception est " => {
                (self.json()?["compteurs"]["reception"].to_string(), "")
            }
            "le total prévu par l’import est " => {
                (self.json()?["couverture"]["attendus"].to_string(), "")
            }
            "le nombre de lettres indexées est " => {
                (self.json()?["couverture"]["indexes"].to_string(), "")
            }
            "la page propose " => (ids(&self.json()?)?.len().to_string(), " fils"),
            _ => return Ok(false),
        };
        let key = text
            .strip_suffix(suffixe)
            .and_then(|t| parameter(t, prefix))
            .ok_or_else(|| Problem::Step("Valeur de page absente.".into()))?;
        expect(valeur.as_str(), example_value(row, key)?)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::RawQuery,
        http::header::SET_COOKIE,
        routing::{get, post},
    };
    use serde_json::json;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn compare_les_deux_sections_et_attend_le_lot_demande() {
        let appels = Arc::new(AtomicUsize::new(0));
        let compteur = appels.clone();
        let pages = get(move |RawQuery(query): RawQuery| {
            let appel = compteur.fetch_add(1, Ordering::SeqCst);
            async move {
                let ids = match query.as_deref() {
                    Some("taille=2") if appel == 0 => {
                        return Json(json!({"nouveaux":[],"deja_vus":[]}));
                    }
                    Some("taille=2") => ["a", "b"],
                    Some("taille=2&curseur=suite") => ["c", "d"],
                    None => ["e", "f"],
                    _ => panic!("Requête de page inattendue"),
                };
                Json(
                    json!({"nouveaux":[{"id":ids[0]}],"deja_vus":[{"id":ids[1]}],
                    "curseur_suivant":"suite","expediteurs_en_attente":3,"compteurs":{"reception":8},
                    "couverture":{"attendus":9,"indexes":8,"import_en_cours":appel<4}}),
                )
            }
        });
        let serveur = Router::new().route("/api/v1/views/reception", pages).route(
            "/api/v1/session",
            post(async || ([(SET_COOKIE, "boite=essai")], Json(json!({})))),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut world = World::new(listener.local_addr().unwrap()).unwrap();
        let service = tokio::spawn(async move { axum::serve(listener, serveur).await.unwrap() });
        let mut row = Example::new();
        world.open_mailbox("alice", "essai").await.unwrap();
        world
            .step("je consulte deux pages consécutives de ma réception", &row)
            .await
            .unwrap();
        for (assertion, valeur) in [
            ("chaque page contient <n> fils", "2"),
            ("les deux pages présentent <n> fils distincts", "4"),
            ("la réception indique <n> expéditeurs en attente", "3"),
            ("le nombre de fils en Réception est <n>", "8"),
            ("le total prévu par l’import est <n>", "9"),
            ("le nombre de lettres indexées est <n>", "8"),
            ("la page propose <n> fils", "2"),
        ] {
            row.insert("n".into(), valeur.into());
            world.step(assertion, &row).await.unwrap();
            row.insert("n".into(), "111".into());
            assert!(world.step(assertion, &row).await.is_err(), "{assertion}");
        }
        assert!(
            world
                .verifier_page("chaque page contient <n> lettres", &row)
                .is_err()
        );
        assert!(
            world
                .verifier_page("la page propose <n> autres", &row)
                .is_err()
        );
        world
            .step("je consulte la réception pendant l’import", &row)
            .await
            .unwrap();
        assert_eq!(world.json().unwrap()["couverture"]["import_en_cours"], true);
        world.lire_pendant_import(false).await.unwrap();
        assert_eq!(
            world.json().unwrap()["couverture"]["import_en_cours"],
            false
        );
        assert!(
            world
                .step(
                    "je rattache une boîte dont le second lot est suspendu",
                    &row
                )
                .await
                .is_err()
        );
        assert!(
            world
                .step("le service reprend le lot suspendu", &row)
                .await
                .is_err()
        );
        assert!(ids(&json!({"nouveaux":[],"deja_vus":[{}]})).is_err());
        assert!(ids(&json!({"nouveaux":[]})).is_err());
        service.abort();
    }
}
