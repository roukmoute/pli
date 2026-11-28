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
    let url = format!("{}/api/v1/session", service.base);
    install_bench(&client, &service.base, admin).await?;
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

// Met en service le profil du banc, sur les ports TLS que Docker lui a attribués.
async fn install_bench(client: &reqwest::Client, base: &str, admin: &str) -> Result<(), Error> {
    let servers = format!("{base}/api/admin/v1/servers");
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
    Ok(())
}

const CAROL: &str = "carol@recette.test";
const ATTENTE: Duration = Duration::from_secs(15);

// Interroge le service jusqu’à ce que la réponse satisfasse la condition.
async fn attendre(
    client: &reqwest::Client,
    url: &str,
    cookie: &str,
    prete: impl Fn(&Value) -> bool,
) -> Result<Value, Error> {
    let lecture = async {
        loop {
            let reponse: Value = client
                .get(url)
                .header("cookie", cookie)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            if prete(&reponse) {
                return Ok::<_, Error>(reponse);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    tokio::time::timeout(ATTENTE, lecture).await?
}

#[tokio::test]
#[ignore = "requiert les boîtes fictives et les ports TLS du banc"]
async fn greenmail_archiver_range_deux_lettres_sous_les_uid_annonces() -> Result<(), Error> {
    // Chaque exécution dépose ses propres lettres dans la boîte de Carol.
    let instant = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let marque = format!("Classement {instant}");
    let port: u16 = std::env::var("PLI_BENCH_IMAP_PORT")?.parse()?;
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
    let mut boite = async_imap::Client::new(stream);
    boite
        .read_response()
        .await?
        .ok_or("Bannière IMAP absente")?;
    let mut boite = boite
        .login(CAROL, "carol-test")
        .await
        .map_err(|(erreur, _)| erreur)?;
    // Le dossier existe déjà quand le test a tourné sur ce banc.
    let _ = boite.create("Archives").await;
    for lettre in ["A", "B"] {
        let brut = format!(
            "From: bob@recette.test\r\nTo: {CAROL}\r\nSubject: {marque} {lettre}\r\n\
             Message-ID: <{lettre}-{instant}@recette.test>\r\n\r\nÀ classer.\r\n"
        );
        boite.append("INBOX", None, None, brut).await?;
    }
    let archivees = boite.examine("Archives").await?.exists;

    let admin = "jeton-de-recette-archivage";
    let dir = tempfile::tempdir()?;
    let service = TestService::start(dir.path(), admin, "1")?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;
    install_bench(&client, &service.base, admin).await?;
    let api = |chemin: &str| format!("{}/api/v1/{chemin}", service.base);
    let session = client
        .post(api("session"))
        .json(&json!({"address": CAROL, "password": "carol-test"}))
        .send()
        .await?
        .error_for_status()?;
    let cookie = session_cookie(&session)?;
    let classer = |page: &Value| -> Vec<String> {
        page["nouveaux"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|fil| {
                fil["objet"]
                    .as_str()
                    .is_some_and(|o| o.starts_with(&marque))
            })
            .filter_map(|fil| fil["id"].as_str().map(str::to_owned))
            .collect()
    };
    let reception = api("views/reception");
    let importee = attendre(&client, &reception, &cookie, |page| {
        classer(page).len() == 2
    })
    .await?;
    let fils = classer(&importee);

    // Les deux lettres partent dans la même commande de déplacement.
    let id = format!(
        "{:08x}-0000-7000-8000-{:012x}",
        instant as u32,
        instant as u64 >> 16
    );
    let operations = api("operations");
    client
        .post(&operations)
        .header("cookie", &cookie)
        .json(&json!({"id": id, "type": "archiver", "vue": "reception", "cibles": fils}))
        .send()
        .await?
        .error_for_status()?;
    let suivi = format!("{operations}/{id}");
    let conclue = attendre(&client, &suivi, &cookie, |o| o["etat"] != "en_attente").await?;
    assert_eq!(conclue["etat"], "confirmee");
    let confirmes = conclue["resultats"]
        .as_array()
        .ok_or("Résultats absents")?
        .iter()
        .filter(|cible| cible["etat"] == "confirme")
        .count();
    assert_eq!(confirmes, fils.len());
    let restante = attendre(&client, &reception, &cookie, |_| true).await?;
    assert!(classer(&restante).is_empty());

    // Chaque lettre est rattachée au dossier Archives, sous l’UID que la boîte lui a donné.
    let dossiers = attendre(&client, &api("folders"), &cookie, |_| true).await?;
    let archives = dossiers["folders"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|dossier| dossier["role"] == "archive")
        .ok_or("Dossier d’archives absent")?["id"]
        .as_i64()
        .ok_or("Identifiant du dossier absent")?;
    let mut rattachees = std::collections::HashSet::new();
    for fil in &fils {
        let url = api(&format!("threads/{}", fil.replace('/', "%2F")));
        let lettres = attendre(&client, &url, &cookie, |_| true).await?;
        let cle = lettres[0]["id"]
            .as_str()
            .ok_or("Lettre absente")?
            .to_owned();
        let (dossier, reste) = cle.split_once('-').ok_or("Clé de lettre illisible")?;
        assert_eq!(dossier, archives.to_string());
        let (_, uid) = reste.split_once('-').ok_or("UID de lettre absent")?;
        rattachees.insert(uid.parse::<u32>()?);
    }
    let recherche = format!("SUBJECT \"{marque}\"");
    assert_eq!(boite.examine("Archives").await?.exists, archivees + 2);
    assert_eq!(boite.uid_search(&recherche).await?, rattachees);
    boite.examine("INBOX").await?;
    assert!(boite.uid_search(&recherche).await?.is_empty());
    boite.logout().await?;
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
