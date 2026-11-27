use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use pli_acceptance::Error;
use serde_json::{Value, json};

const GENERATOR: &str = env!("CARGO_BIN_EXE_acceptance-entrypoint-generator");
const RUNNER: &str = env!("CARGO_BIN_EXE_acceptance-runner");

#[test]
fn generated_entrypoint_runs_each_example_and_worker_reuses_it_with_another_ir() -> Result<(), Error>
{
    let dir = tempfile::tempdir()?;
    let base = dir.path().join("disponibilité.json");
    let suite = dir.path().join("suite");
    let mut feature = json!({
        "name": "Santé",
        "background": [{"keyword":"When", "text":"je consulte la disponibilité du webmail"}],
        "scenarios": [
            {"name":"Simple", "steps":null, "examples":[]},
            {"name":"Disponibilité", "steps":[{"keyword":"Then", "text":"l’état annoncé est <etat>"}],
             "examples":[{"etat":"ok"}, {"etat":"ok"}]}
        ]
    });
    fs::write(&base, feature.to_string())?;
    assert!(
        Command::new(GENERATOR)
            .arg(&base)
            .arg(&suite)
            .status()?
            .success()
    );
    let bin_dir = Path::new(RUNNER).parent().unwrap();
    let baseline = Command::new(suite.join("run"))
        .env("PATH", bin_dir)
        .env_remove("PLI_ACCEPTANCE_IR")
        .output()?;
    assert!(
        baseline.status.success(),
        "{}",
        String::from_utf8_lossy(&baseline.stderr)
    );
    assert_eq!(
        String::from_utf8(baseline.stdout)?
            .lines()
            .collect::<Vec<_>>(),
        [
            "OK   Santé / Simple / exemple 1",
            "OK   Santé / Disponibilité / exemple 1",
            "OK   Santé / Disponibilité / exemple 2"
        ]
    );

    feature["scenarios"][1]["examples"][1]["etat"] = json!("indisponible");
    let mutant = dir.path().join("mutant.json");
    fs::write(&mutant, feature.to_string())?;
    feature["scenarios"][1]["examples"] = json!([]);
    let incompatible = dir.path().join("structure.json");
    fs::write(&incompatible, feature.to_string())?;

    let mut worker = Command::new(RUNNER)
        .arg("--worker")
        .env("PATH", bin_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut input = worker.stdin.take().unwrap();
    for (id, ir) in [
        ("initial", &base),
        ("mutant", &mutant),
        ("structure", &incompatible),
    ] {
        writeln!(
            input,
            "{}",
            json!({"id":id, "generated_dir":suite, "feature_json":ir})
        )?;
    }
    drop(input);
    let output = worker.wait_with_output()?;
    assert!(output.status.success());
    let replies: Vec<Value> = String::from_utf8(output.stdout)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0]["outcome"], "test_success");
    assert_eq!(replies[1]["outcome"], "test_failure");
    assert!(
        replies[1]["output"].as_str().unwrap().contains(
            "exemple 2 — l’état annoncé est <etat> — Attendu « indisponible », reçu « ok »"
        )
    );
    assert_eq!(replies[2]["outcome"], "infrastructure_error");
    Ok(())
}

#[test]
fn generator_distinguishes_usage_errors_from_an_empty_feature() -> Result<(), Error> {
    let dir = tempfile::tempdir()?;
    let ir = dir.path().join("vide.json");
    fs::write(&ir, r#"{"name":"Vide","scenarios":[]}"#)?;
    assert_eq!(Command::new(GENERATOR).output()?.status.code(), Some(2));
    let empty = Command::new(GENERATOR).arg(ir).arg(dir.path()).output()?;
    assert_eq!(empty.status.code(), Some(1));
    assert!(String::from_utf8(empty.stderr)?.contains("Aucun scénario dans Vide"));
    Ok(())
}
