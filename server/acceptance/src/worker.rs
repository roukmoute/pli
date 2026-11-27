use std::{
    io::{BufRead, Write},
    path::PathBuf,
    process::{Output, Stdio},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::{process::Command, time::timeout};

use crate::{ENTRYPOINT, Error, IR_ENV};

#[derive(Deserialize)]
struct Job {
    id: String,
    feature_json: PathBuf,
    generated_dir: PathBuf,
    timeout: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    TestSuccess,
    TestFailure,
    InfrastructureError,
}

#[derive(Serialize)]
struct Reply {
    id: String,
    outcome: Outcome,
    output: String,
    error: String,
    duration: u64,
}

async fn launch(job: &Job) -> Result<Output, Error> {
    let limit = job
        .timeout
        .as_deref()
        .map(humantime::parse_duration)
        .transpose()?
        .unwrap_or(Duration::from_secs(30));
    let mut command = Command::new(job.generated_dir.join(ENTRYPOINT));
    command
        .env(IR_ENV, &job.feature_json)
        .stdin(Stdio::null())
        .kill_on_drop(true);
    Ok(timeout(limit, command.output())
        .await
        .map_err(|_| format!("Délai dépassé ({limit:?})"))??)
}

async fn reply(job: Job) -> Reply {
    let start = Instant::now();
    let (outcome, output, error) = match launch(&job).await {
        Ok(result) => {
            let output = format!(
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            match result.status.code() {
                Some(0) => (Outcome::TestSuccess, output, String::new()),
                Some(1) => (Outcome::TestFailure, output, String::new()),
                _ => (
                    Outcome::InfrastructureError,
                    output,
                    format!("Arrêt du lanceur ({})", result.status),
                ),
            }
        }
        Err(e) => (Outcome::InfrastructureError, String::new(), e.to_string()),
    };
    Reply {
        id: job.id,
        outcome,
        output,
        error,
        duration: start.elapsed().as_nanos() as u64,
    }
}

pub async fn serve(input: impl BufRead, mut output: impl Write) -> Result<(), Error> {
    for line in input.lines() {
        let job = serde_json::from_str(&line?)?;
        serde_json::to_writer(&mut output, &reply(job).await)?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    use serde_json::{Value, json};

    use super::*;

    #[tokio::test]
    async fn answers_success_failure_and_infrastructure_jobs_on_the_same_stream()
    -> Result<(), Error> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("run");
        fs::write(
            &path,
            "#!/bin/sh\necho \"IR=$PLI_ACCEPTANCE_IR\"\necho diagnostic >&2\nexit \"$PLI_ACCEPTANCE_IR\"\n",
        )?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        let mut jobs = Vec::new();
        for (id, folder) in [
            ("0", dir.path()),
            ("1", dir.path()),
            ("2", dir.path()),
            ("3", &dir.path().join("absent")),
        ] {
            writeln!(
                jobs,
                "{}",
                json!({"id":id,"generated_dir":folder,"feature_json":id})
            )?;
        }
        let mut output = Vec::new();
        serve(jobs.as_slice(), &mut output).await?;
        let responses: Vec<Value> = serde_json::Deserializer::from_slice(&output)
            .into_iter()
            .collect::<Result<_, _>>()?;
        assert_eq!(responses.len(), 4);
        for (i, outcome) in [
            "test_success",
            "test_failure",
            "infrastructure_error",
            "infrastructure_error",
        ]
        .iter()
        .enumerate()
        {
            let r = &responses[i];
            assert_eq!(r["id"], i.to_string());
            assert_eq!(r["outcome"], *outcome);
            assert!(r["duration"].is_u64());
            if i < 3 {
                assert_eq!(r["output"], format!("IR={i}\ndiagnostic\n"));
            }
            assert_eq!(r["error"] == "", i < 2);
        }
        Ok(())
    }

    #[tokio::test]
    async fn timeout_stops_the_test_process_and_reports_an_infrastructure_error()
    -> Result<(), Error> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("run");
        let pid_file = dir.path().join("pid");
        fs::write(
            &path,
            "#!/bin/sh\necho $$ > \"$PLI_ACCEPTANCE_IR\"\nexec sleep 5\n",
        )?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        let result = reply(Job {
            id: "m1".into(),
            generated_dir: dir.path().into(),
            feature_json: pid_file.clone(),
            timeout: Some("300ms".into()),
        })
        .await;
        assert!(matches!(result.outcome, Outcome::InfrastructureError));
        assert!(result.error.contains("Délai dépassé"));
        let pid = fs::read_to_string(pid_file)?;
        timeout(Duration::from_secs(2), async {
            while Command::new("kill")
                .args(["-0", pid.trim()])
                .output()
                .await?
                .status
                .success()
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Ok::<_, std::io::Error>(())
        })
        .await??;
        Ok(())
    }
}
