use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
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

struct TestService {
    process: ServerProcess,
    stdout: BufReader<ChildStdout>,
    log: String,
    base: String,
}

impl TestService {
    fn start(dir: &Path, token: &str, flag: &str) -> Result<Self, Error> {
        let mut process = ServerProcess(
            Command::new(env!("CARGO_BIN_EXE_webmail-server"))
                .env("WEBMAIL_LISTEN", "127.0.0.1:0")
                .env("WEBMAIL_DATA_DIR", dir)
                .env("WEBMAIL_ADMIN_TOKEN", token)
                .env("WEBMAIL_ALLOW_TEST_PROFILE", flag)
                .env("RUST_LOG", "trace")
                .env("NO_COLOR", "1")
                .env("WEBMAIL_SECRET_KEY", hex::encode([42; 32]))
                .env("WEBMAIL_SESSION_TTL_SECONDS", "300")
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
        let base = format!("http://{address}");
        Ok(Self {
            process,
            stdout,
            log,
            base,
        })
    }

    fn finish(mut self) -> Result<String, Error> {
        self.process.0.kill()?;
        self.process.0.wait()?;
        self.stdout.read_to_string(&mut self.log)?;
        self.process
            .0
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut self.log)?;
        Ok(self.log)
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
        let service = TestService::start(dir.path(), token, flag)?;
        let url = format!("{}/api/admin/v1/servers", service.base);
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
        let log = service.finish()?;
        assert!(!log.contains(token));
        assert!(!log.contains(password));
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requiert les boîtes fictives et les ports TLS du banc"]
async fn greenmail_sessions_isolate_accounts_and_keep_credentials_out_of_storage_and_logs()
-> Result<(), Error> {
    let admin = "jeton-de-recette-session";
    let dir = tempfile::tempdir()?;
    let service = TestService::start(dir.path(), admin, "1")?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;
    let servers = format!("{}/api/admin/v1/servers", service.base);
    let url = format!("{}/api/v1/session", service.base);
    let catalog: Value = client
        .get(&servers)
        .bearer_auth(admin)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut profile = catalog["presets"][1].clone();
    assert_eq!(profile["imap"]["host"], "127.0.0.1");
    assert_eq!(profile["smtp"]["host"], "127.0.0.1");
    profile["imap"]["port"] = json!(std::env::var("PLI_BENCH_IMAPS_PORT")?.parse::<u16>()?);
    profile["smtp"]["port"] = json!(std::env::var("PLI_BENCH_SMTPS_PORT")?.parse::<u16>()?);
    let credentials = json!({"username":"alice@recette.test","password":"alice-test"});
    client
        .post(&servers)
        .bearer_auth(admin)
        .json(&json!({"profile":profile,"credentials":credentials}))
        .send()
        .await?
        .error_for_status()?;
    let login = |address, password| {
        client
            .post(&url)
            .json(&json!({"address":address,"password":password}))
            .send()
    };
    let bad = login("alice@recette.test", "incorrect-test").await?;
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(bad.json::<Value>().await?["code"], "identifiants_invalides");
    let alice = login("alice@recette.test", "alice-test")
        .await?
        .error_for_status()?;
    let alice_cookie = session_cookie(&alice)?;
    let alice_account = alice.json::<Value>().await?;
    let bob = login("bob@recette.test", "bob-test")
        .await?
        .error_for_status()?;
    let bob_cookie = session_cookie(&bob)?;
    let alice_id = alice_account["account_id"]
        .as_i64()
        .ok_or("Compte d’Alice absent")?;
    let as_bob: Value = client
        .get(format!("{url}?account_id={alice_id}"))
        .header("cookie", &bob_cookie)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_ne!(as_bob["account_id"], alice_id);
    assert_eq!(as_bob["address"], "bob@recette.test");
    assert!(!as_bob.to_string().contains("alice"));
    let folders_url = format!("{}/api/v1/folders?account_id={alice_id}", service.base);
    let alice_folders: Value = client
        .get(&folders_url)
        .header("cookie", &alice_cookie)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let folders = alice_folders["folders"]
        .as_array()
        .ok_or("Dossiers absents")?;
    for (path, role) in [
        ("INBOX", Some("inbox")),
        ("Gesendet", Some("sent")),
        ("Papierkorb", Some("trash")),
        ("Atelier", None),
    ] {
        let folder = folders
            .iter()
            .find(|folder| folder["path"] == path)
            .ok_or("Dossier du banc absent")?;
        assert_eq!(folder.get("role"), Some(&json!(role)));
    }
    let bob_folders: Value = client
        .get(&folders_url)
        .header("cookie", &bob_cookie)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let folders = bob_folders["folders"]
        .as_array()
        .ok_or("Dossiers de Bob absents")?;
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0]["path"], "INBOX");
    assert_ne!(
        folders[0]["id"],
        alice_folders["folders"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["path"] == "INBOX")
            .unwrap()["id"]
    );
    let logout = client
        .delete(&url)
        .header("cookie", &alice_cookie)
        .send()
        .await?;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        client
            .get(&url)
            .header("cookie", &alice_cookie)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .get(&url)
            .header("cookie", &bob_cookie)
            .send()
            .await?
            .status(),
        StatusCode::OK
    );
    let log = service.finish()?;
    let tokens = [alice_cookie, bob_cookie];
    let mut private = vec!["alice-test", "bob-test", "incorrect-test", admin];
    for cookie in &tokens {
        private.push(
            cookie
                .strip_prefix("__Host-pli_session=")
                .ok_or("Cookie de session absent")?,
        );
    }
    for path in dir.path().read_dir()? {
        let bytes = std::fs::read(path?.path())?;
        for value in &private {
            assert!(
                !bytes.windows(value.len()).any(|w| w == value.as_bytes()),
                "Donnée privée présente dans SQLite"
            );
        }
    }
    private.extend(["alice@recette.test", "bob@recette.test"]);
    for value in private {
        assert!(
            !log.contains(value),
            "Donnée privée présente dans le journal"
        );
    }
    Ok(())
}

fn session_cookie(response: &reqwest::Response) -> Result<String, Error> {
    Ok(response
        .headers()
        .get("set-cookie")
        .ok_or("Cookie absent")?
        .to_str()?
        .split_once(';')
        .ok_or("Attributs du cookie absents")?
        .0
        .to_owned())
}
