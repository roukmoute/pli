use std::{fs, path::Path};

use super::{Example, Problem, World, example_value, expect, parameter};

fn has_secret(directory: &Path) -> Result<bool, Problem> {
    let needle = b"alice-test";
    let contains = |data: Vec<u8>| data.windows(needle.len()).any(|part| part == needle);
    let db = fs::read(directory.join("webmail.db")).map_err(Problem::infra)?;
    let wal = match fs::read(directory.join("webmail.db-wal")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        file => file.map_err(Problem::infra)?,
    };
    Ok(contains(db) || contains(wal))
}

impl World {
    pub(super) fn privacy_step(&self, text: &str, row: &Example) -> Result<bool, Problem> {
        let Some((prefix, _)) = text.split_once('<') else {
            return Ok(false);
        };
        let (needle, present, absent) = match prefix {
            "le secret en clair dans les fichiers de la boîte est " => (None, "présent", "absent"),
            "le secret dans le journal est " => (Some("alice-test"), "présent", "absent"),
            "l’adresse complète dans le journal est " => {
                (Some("alice@recette.test"), "présente", "absente")
            }
            _ => return Ok(false),
        };
        let (directory, log) = self
            .recordings
            .as_ref()
            .ok_or_else(|| Problem::infra("Fichiers de recette absents."))?;
        let found = if let Some(needle) = needle {
            let log = fs::read_to_string(log).map_err(Problem::infra)?;
            if !log.contains("Serveur prêt") {
                return Err(Problem::infra(
                    "Aucun démarrage dans le journal de recette.",
                ));
            }
            log.contains(needle)
        } else {
            has_secret(directory)?
        };
        let key = parameter(text, prefix)
            .ok_or_else(|| Problem::Step("Paramètre de confidentialité absent.".into()))?;
        expect(
            if found { present } else { absent },
            example_value(row, key)?,
        )?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_assertions_notice_a_leak_in_either_database_file_or_in_the_log() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("webmail.db");
        let wal = dir.path().join("webmail.db-wal");
        let log = dir.path().join("journal");
        let mut world = World::new("127.0.0.1:1".parse().unwrap()).unwrap();
        world.recordings = Some((dir.path().into(), log.clone()));
        fs::write(&db, b"base").unwrap();
        fs::write(&log, "Serveur prêt").unwrap();
        let verify = |step: &str, value: &str| {
            world
                .privacy_step(
                    &format!("{step} est <valeur>"),
                    &Example::from([("valeur".into(), value.into())]),
                )
                .unwrap()
        };
        let storage = "le secret en clair dans les fichiers de la boîte";
        assert!(verify(storage, "absent"));
        fs::write(&wal, b"alice-test").unwrap();
        assert!(verify(storage, "présent"));
        fs::write(&wal, []).unwrap();
        fs::write(&db, b"alice-test").unwrap();
        assert!(verify(storage, "présent"));
        assert!(verify("le secret dans le journal", "absent"));
        assert!(verify("l’adresse complète dans le journal", "absente"));
        fs::write(&log, "Serveur prêt\nalice-test alice@recette.test").unwrap();
        assert!(verify("le secret dans le journal", "présent"));
        assert!(verify("l’adresse complète dans le journal", "présente"));
    }
}
