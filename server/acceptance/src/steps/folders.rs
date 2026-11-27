use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::{Example, Problem, World, example_value, expect, parameter};

impl World {
    fn database(&self) -> Result<Connection, Problem> {
        let (directory, _) = self
            .recordings
            .as_ref()
            .ok_or_else(|| Problem::infra("Base de recette absente."))?;
        Connection::open_with_flags(
            directory.join("webmail.db"),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(Problem::infra)
    }

    fn alice_marker(&self) -> Result<(u32, u32), Problem> {
        self.database()?.query_row(
            "SELECT m.uidvalidity, m.uidnext FROM inbox_markers m JOIN accounts a ON a.id=m.account_id
             WHERE a.address='alice@recette.test'", [], |r| Ok((r.get(0)?, r.get(1)?))
        ).map_err(|_| Problem::Step("Repère de réception d’Alice absent.".into()))
    }

    fn imported_messages(&self) -> Result<u32, Problem> {
        let db = self.database()?;
        if !db.table_exists(None, "messages").map_err(Problem::infra)? {
            return Ok(0);
        }
        db.query_row(
            "SELECT count(*) FROM messages m JOIN accounts a ON a.id=m.account_id
                      WHERE a.address='alice@recette.test'",
            [],
            |r| r.get(0),
        )
        .map_err(Problem::infra)
    }

    async fn restart_service(&mut self) -> Result<(), Problem> {
        let (reply, address) = tokio::sync::oneshot::channel();
        self.restart
            .as_ref()
            .ok_or_else(|| Problem::infra("Service de recette absent."))?
            .send(reply)
            .await
            .map_err(Problem::infra)?;
        self.address = address.await.map_err(Problem::infra)?;
        Ok(())
    }

    pub(super) async fn folder_step(&mut self, text: &str, row: &Example) -> Result<bool, Problem> {
        let query = match text {
            "je consulte les dossiers de ma boîte" => String::new(),
            "Bob consulte les dossiers en indiquant le compte d’Alice" => {
                format!("?account_id={}", self.session.alice_id()?)
            }
            "le service redémarre et je retrouve mes dossiers" => {
                self.restart_service().await?;
                String::new()
            }
            _ => return self.folder_assertion(text, row),
        };
        self.read_current(format!("http://{}/api/v1/folders{query}", self.address))
            .await?;
        Ok(true)
    }

    fn folder_assertion(&mut self, text: &str, row: &Example) -> Result<bool, Problem> {
        let (prefix, _) = text.split_once('<').unwrap_or((text, ""));
        let value = match prefix {
            "le dossier des envoyés est " => paths(&self.json()?, Some("sent"))?,
            "le dossier de la corbeille est " => paths(&self.json()?, Some("trash"))?,
            "les dossiers personnels sont " => paths(&self.json()?, Some("personnel"))?,
            "les chemins des dossiers consultables sont " => paths(&self.json()?, None)?,
            "le prochain numéro de réception au rattachement est " => {
                let marker = self.alice_marker()?;
                self.initial_marker = Some(marker);
                marker.1.to_string()
            }
            "le nombre de messages déjà importés est " => self.imported_messages()?.to_string(),
            "le repère de réception depuis le rattachement est " => {
                let first = self
                    .initial_marker
                    .ok_or_else(|| Problem::Step("Premier repère absent.".into()))?;
                if self.alice_marker()? == first {
                    "identique"
                } else {
                    "modifié"
                }
                .into()
            }
            _ => return Ok(false),
        };
        let key = parameter(text, prefix)
            .ok_or_else(|| Problem::Step("Paramètre de dossier absent.".into()))?;
        expect(value.as_str(), example_value(row, key)?)?;
        Ok(true)
    }
}

fn paths(body: &Value, role: Option<&str>) -> Result<String, Problem> {
    let folders = body["folders"]
        .as_array()
        .ok_or_else(|| Problem::Step("Liste des dossiers absente.".into()))?;
    let mut paths = Vec::new();
    for folder in folders {
        let actual = folder
            .get("role")
            .ok_or_else(|| Problem::Step("Rôle du dossier absent.".into()))?;
        let actual = match actual {
            Value::String(role) => role.as_str(),
            Value::Null => "personnel",
            _ => return Err(Problem::Step("Rôle de dossier illisible.".into())),
        };
        if role.is_none_or(|role| actual == role) {
            paths.push(
                folder["path"]
                    .as_str()
                    .ok_or_else(|| Problem::Step("Chemin du dossier absent.".into()))?,
            );
        }
    }
    paths.sort_unstable();
    Ok(paths.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use serde_json::json;

    #[tokio::test]
    async fn folder_steps_compare_the_cookie_response_and_the_committed_marker() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(dir.path().join("webmail.db")).unwrap();
        db.execute_batch("CREATE TABLE accounts(id, address); INSERT INTO accounts VALUES(3,'alice@recette.test');
            CREATE TABLE inbox_markers(account_id,uidvalidity,uidnext); INSERT INTO inbox_markers VALUES(3,19,25);").unwrap();
        let body = json!({"folders":[{"path":"P", "role":"trash"},{"path":"A", "role":null},{"path":"E", "role":"sent"}]});
        let mut world = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        world.recordings = Some((dir.path().into(), dir.path().join("log")));
        world.response = Some((StatusCode::OK, body.to_string()));
        for (prefix, expected) in [
            ("le dossier des envoyés est ", "E"),
            ("le dossier de la corbeille est ", "P"),
            ("les dossiers personnels sont ", "A"),
            ("les chemins des dossiers consultables sont ", "A,E,P"),
            ("le prochain numéro de réception au rattachement est ", "25"),
            ("le nombre de messages déjà importés est ", "0"),
            (
                "le repère de réception depuis le rattachement est ",
                "identique",
            ),
        ] {
            for (value, passes) in [(expected, true), ("autre", false)] {
                let row = Example::from([("attendu".into(), value.into())]);
                let result = world.folder_step(&format!("{prefix}<attendu>"), &row).await;
                assert_eq!(matches!(result, Ok(true)), passes, "{prefix}{value}");
            }
        }
        db.execute_batch(
            "CREATE TABLE messages(account_id); INSERT INTO messages VALUES(3),(4);
            UPDATE inbox_markers SET uidvalidity=20;",
        )
        .unwrap();
        assert_eq!(world.imported_messages().unwrap(), 1);
        let expected = Example::from([("repere".into(), "identique".into())]);
        assert!(
            world
                .folder_step(
                    "le repère de réception depuis le rattachement est <repere>",
                    &expected
                )
                .await
                .is_err()
        );
        for invalid in [
            json!({}),
            json!({"folders":[{}]}),
            json!({"folders":[{"role":3}]}),
            json!({"folders":[{"role":null}]}),
        ] {
            assert!(paths(&invalid, None).is_err());
        }
    }
}
