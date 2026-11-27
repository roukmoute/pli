use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{Case, ENTRYPOINT, Error, Feature, INTERPRETER};

#[derive(Deserialize, Serialize)]
pub(crate) struct Entrypoint {
    pub ir_path: PathBuf,
    pub cases: Vec<Case>,
}

impl Entrypoint {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let script = fs::read_to_string(path)?;
        let data = script
            .strip_prefix(INTERPRETER)
            .ok_or("Point d’entrée APS invalide")?;
        Ok(serde_json::from_str(data)?)
    }
}

fn metadata_name(feature: &str) -> String {
    let parts: Vec<_> = feature
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    format!("{}.json", parts.join("-").to_ascii_lowercase())
}

pub fn generate(ir: &Path, output: &Path) -> Result<(), Error> {
    let feature = Feature::load(ir)?;
    let cases = feature.cases();
    if cases.is_empty() {
        return Err(format!("Aucun scénario dans {}", feature.name).into());
    }
    let entry = Entrypoint {
        ir_path: ir.into(),
        cases,
    };
    let script = format!("{INTERPRETER}{}\n", serde_json::to_string_pretty(&entry)?);
    let path = output.join(ENTRYPOINT);
    let metadata = output.join("metadata");
    fs::create_dir_all(&metadata)?;
    fs::write(&path, &script)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;

    let source = Path::new("features")
        .join(ir.file_name().ok_or("Nom de fichier IR manquant")?)
        .with_extension("feature");
    let source = source.to_str().ok_or("Nom de fonctionnalité non UTF-8")?;
    let hash = hex::encode(Sha256::digest(script.as_bytes()));
    let info = json!({
        "schema_version": 1, "feature_path": source, "ir_path": ir,
        "implementation_hash": format!("sha256:{hash}"), "hash_scope": "generated_files",
        "generated_files": [path]
    });
    fs::write(
        metadata.join(metadata_name(source)),
        serde_json::to_vec_pretty(&info)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_each_case_and_hashes_only_the_executable() -> Result<(), Error> {
        let dir = tempfile::tempdir()?;
        let ir = dir.path().join("Santé & API.json");
        let output = dir.path().join("generated");
        let mut feature = json!({"name":"Santé","scenarios":[
            {"name":"simple","steps":[],"examples":[]},
            {"name":"table","steps":[],"examples":[{"etat":"ok"},{"etat":"autre"}]}
        ]});
        fs::write(&ir, feature.to_string())?;
        generate(&ir, &output)?;
        let path = output.join("run");
        let generated = fs::read(&path)?;
        assert!(generated.starts_with(b"#!/usr/bin/env acceptance-runner\n"));
        assert_ne!(path.metadata()?.permissions().mode() & 0o111, 0);
        let entry = Entrypoint::load(&path)?;
        assert_eq!(entry.ir_path, ir);
        assert_eq!(entry.cases, [(0, 0), (1, 0), (1, 1)]);
        let metadata = output.join("metadata/features-sant-api-feature.json");
        let info: serde_json::Value = serde_json::from_slice(&fs::read(&metadata)?)?;
        assert_eq!(
            info,
            json!({
                "schema_version":1, "feature_path":"features/Santé & API.feature", "ir_path":ir,
                "implementation_hash":format!("sha256:{}", hex::encode(Sha256::digest(&generated))),
                "hash_scope":"generated_files", "generated_files":[path]
            })
        );
        feature["scenarios"][1]["examples"][0]["etat"] = json!("modifié");
        fs::write(&ir, feature.to_string())?;
        generate(&ir, &output)?;
        assert_eq!(fs::read(path)?, generated);
        Ok(())
    }
}
