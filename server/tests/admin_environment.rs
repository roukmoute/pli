use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    time::Duration,
};

use reqwest::StatusCode;
use serde_json::{Value, json};
use webmail_server::Error;

struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn only_the_explicit_test_flag_enables_the_bench_and_secrets_stay_out_of_logs()
-> Result<(), Error> {
    let token = "jeton-fictif-confidentiel";
    let password = "mot-de-passe-fictif-confidentiel";
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;
    for (flag, expected_presets) in [("1", 2), ("0", 1), ("true", 1)] {
        let dir = tempfile::tempdir()?;
        let mut process = ServerProcess(
            Command::new(env!("CARGO_BIN_EXE_webmail-server"))
                .env("WEBMAIL_LISTEN", "127.0.0.1:0")
                .env("WEBMAIL_DATA_DIR", dir.path())
                .env("WEBMAIL_ADMIN_TOKEN", token)
                .env("WEBMAIL_ALLOW_TEST_PROFILE", flag)
                .env("RUST_LOG", "trace")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?,
        );
        let mut stdout = BufReader::new(process.0.stdout.take().unwrap());
        let mut log = String::new();
        stdout.read_line(&mut log)?;
        let (_, address) = log
            .trim_end()
            .rsplit_once("address=")
            .ok_or("Adresse absente")?;
        let url = format!("http://{address}/api/admin/v1/servers");
        assert_eq!(
            client.get(&url).send().await?.status(),
            StatusCode::UNAUTHORIZED
        );
        let catalog: Value = client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_eq!(
            catalog["presets"].as_array().unwrap().len(),
            expected_presets
        );
        let mut profile = catalog["presets"][0].clone();
        profile["imap"]["host"] = json!("invalide.invalid");
        profile["smtp"]["host"] = json!("invalide.invalid");
        let body =
            json!({"profile":profile, "credentials":{"username":"essai", "password":password}});
        let result = client
            .post(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;
        assert_eq!(result.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let diagnostic = result.text().await?;
        let rejected: Value = serde_json::from_str(&diagnostic)?;
        assert_eq!(rejected["code"], "diagnostic_failed");
        assert!(!diagnostic.contains(password));
        process.0.kill()?;
        process.0.wait()?;
        stdout.read_to_string(&mut log)?;
        process.0.stderr.take().unwrap().read_to_string(&mut log)?;
        assert!(!log.contains(token));
        assert!(!log.contains(password));
    }
    Ok(())
}
