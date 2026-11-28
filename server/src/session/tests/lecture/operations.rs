use std::time::Duration;

use tokio::{sync::mpsc, task::JoinHandle, time::timeout};

use super::*;

pub(super) const OPERATIONS: &str = "/api/v1/operations";
const OUVERTURE: &str = "/api/v1/messages/1-1-3/open";
const A: &str = "0190a1b2-0000-7000-8000-00000000000a";
const B: &str = "0190a1b2-0000-7000-8000-00000000000b";
const C: &str = "0190a1b2-0000-7000-8000-00000000000c";
const LETTRE: &str = "1-1-3";
const ETOILEE: &str = r#"["\\Flagged"]"#;

pub(super) struct Banc {
    pub app: TestApp,
    pub router: Router,
    pub cookie: String,
}

// La lettre 3 d’Alice est non lue et porte une étoile.
pub(super) async fn banc() -> Result<(Banc, TcpListener), Error> {
    let app = TestApp::new()?;
    let router = routes(&app, true, config());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let cookie = preparer(&app, listener.local_addr()?.port())?;
    Ok((
        Banc {
            app,
            router,
            cookie,
        },
        listener,
    ))
}

// Faux serveur qui annonce chaque dialogue terminé puis rend son écoute.
pub(super) fn messagerie(
    listener: TcpListener,
    dialogues: Vec<Dialogue>,
) -> (JoinHandle<TcpListener>, mpsc::UnboundedReceiver<()>) {
    let (fini, suivi) = mpsc::unbounded_channel();
    let pair = tokio::spawn(async move {
        for echange in dialogues {
            let socket = listener.accept().await.unwrap().0;
            dialogue(socket, echange).await;
            let _ = fini.send(());
        }
        listener
    });
    (pair, suivi)
}

// Tous les dialogues prévus ont eu lieu et aucune autre connexion n’arrive.
pub(super) async fn sans_autre_commande(pair: JoinHandle<TcpListener>) -> Result<(), Error> {
    let listener = timeout(Duration::from_secs(5), pair).await??;
    assert!(
        timeout(Duration::from_millis(50), listener.accept())
            .await
            .is_err()
    );
    Ok(())
}

fn depot(commande: &str, reponse: &str) -> Dialogue {
    let mut echange = selection("SELECT", 1);
    echange.push(ok(commande, reponse));
    echange
}

fn hors_service() -> Dialogue {
    login_dialog("NO [UNAVAILABLE] backend offline")
}

fn operation(id: &str, genre: &str) -> Value {
    json!({"id": id, "type": genre, "cibles": [LETTRE]})
}

impl Banc {
    pub async fn envoyer(&self, chemin: &str, corps: Value) -> Reply {
        request(
            &self.router,
            Method::POST,
            chemin,
            Some(&self.cookie),
            corps,
        )
        .await
    }

    pub async fn lire(&self, chemin: &str) -> Value {
        request(
            &self.router,
            Method::GET,
            chemin,
            Some(&self.cookie),
            Value::Null,
        )
        .await
        .body
    }

    pub async fn etat(&self, id: &str) -> Value {
        self.lire(&format!("{OPERATIONS}/{id}")).await
    }

    // Attend que la messagerie ait confirmé ou refusé l’opération.
    pub async fn conclue(&self, id: &str) -> Value {
        let attente = async {
            loop {
                let operation = self.etat(id).await;
                if operation["etat"] != "en_attente" {
                    return operation;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        };
        timeout(Duration::from_secs(5), attente)
            .await
            .expect("opération jamais conclue")
    }

    fn drapeaux(&self) -> Result<String, Error> {
        Ok(self.app.db.reader()?.query_row(
            "SELECT flags FROM messages WHERE (account_id,folder_id,uid)=(1,1,3)",
            [],
            |r| r.get(0),
        )?)
    }
}

#[tokio::test]
async fn repond_avant_la_messagerie_puis_rend_le_resultat_connu_au_rejeu() -> Result<(), Error> {
    let (banc, listener) = banc().await?;
    let lue = operation(A, "lu");
    let reponse = banc.envoyer(OPERATIONS, lue.clone()).await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(
        reponse.body,
        json!({
            "id": A, "type": "lu", "etat": "en_attente",
            "revision": reponse.body["revision"],
            "resultats": [{"id": LETTRE, "etat": "en_attente"}]
        })
    );
    // La messagerie n’a encore rien répondu : la liste présente déjà la lettre lue.
    let page = banc.lire("/api/v1/views/reception").await;
    assert_eq!(page["deja_vus"][0]["id"], "1/fil");
    assert_eq!(page["revision"], reponse.body["revision"]);
    assert_eq!(banc.etat(A).await, reponse.body);
    let (pair, _suivi) = messagerie(
        listener,
        vec![depot(
            "UID STORE 3 +FLAGS (\\Seen)",
            "* 3 FETCH (UID 3 FLAGS (\\Seen \\Flagged))\r\n",
        )],
    );
    let confirmee = banc.conclue(A).await;
    assert_eq!(confirmee["etat"], "confirmee");
    assert_eq!(
        confirmee["resultats"],
        json!([{"id": LETTRE, "etat": "confirme"}])
    );
    assert_eq!(confirmee["revision"], reponse.body["revision"]);
    // Le rejeu rend le résultat connu : ni nouvelle opération ni second STORE.
    let rejeu = banc.envoyer(OPERATIONS, lue).await;
    assert_eq!(rejeu.status, StatusCode::OK);
    assert_eq!(rejeu.body, confirmee);
    let operations: i64 =
        banc.app
            .db
            .reader()?
            .query_row("SELECT count(*) FROM operations", [], |r| r.get(0))?;
    assert_eq!(operations, 1);
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn refuse_une_demande_invalide_et_la_lettre_d_un_autre_compte() -> Result<(), Error> {
    let (banc, listener) = banc().await?;
    let (pair, _suivi) = messagerie(listener, Vec::new());
    let sans_session = request(
        &banc.router,
        Method::POST,
        OPERATIONS,
        None,
        operation(A, "lu"),
    )
    .await;
    assert_eq!(sans_session.status, StatusCode::UNAUTHORIZED);
    let trop_long = format!("{A}a");
    for id in [
        "",
        &A[1..],
        &trop_long,
        "0190a1b2x0000-7000-8000-00000000000a",
        "0190a1b2-0000-7000-8000-00000000000g",
    ] {
        let reponse = banc.envoyer(OPERATIONS, operation(id, "lu")).await;
        assert_eq!(reponse.status, StatusCode::BAD_REQUEST, "{id}");
        assert_eq!(reponse.body["code"], "operation_invalide");
    }
    let lot = |taille| json!({"id": A, "type": "lu", "cibles": vec!["x"; taille]});
    for taille in [0, 101] {
        let reponse = banc.envoyer(OPERATIONS, lot(taille)).await;
        assert_eq!(reponse.status, StatusCode::BAD_REQUEST, "{taille}");
    }
    let inconnu = banc.envoyer(OPERATIONS, operation(A, "archive")).await;
    assert_eq!(inconnu.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(inconnu.body["code"], "operation_invalide");
    assert_eq!(banc.etat(A).await["code"], "operation_introuvable");
    // Cent cibles inconnues forment une opération valable, refusée faute de lettre.
    let plein = banc.envoyer(OPERATIONS, lot(100)).await;
    assert_eq!(plein.status, StatusCode::OK);
    assert_eq!(plein.body["etat"], "refusee");
    assert_eq!(plein.body["resultats"].as_array().unwrap().len(), 100);

    let jeton = Token::generate()?;
    store::open(
        &mut *banc.app.db.writer()?,
        2,
        "bob@recette.test".into(),
        &jeton,
        None,
        300,
    )?;
    let bob = format!("{COOKIE_NAME}={}", jeton.encode());
    let etrangere = request(
        &banc.router,
        Method::POST,
        OPERATIONS,
        Some(&bob),
        operation(B, "lu"),
    )
    .await;
    assert_eq!(etrangere.status, StatusCode::OK);
    assert_eq!(etrangere.body["etat"], "refusee");
    assert_eq!(
        etrangere.body["resultats"],
        json!([{"id": LETTRE, "etat": "absent"}])
    );
    assert_eq!(banc.drapeaux()?, ETOILEE);
    // Chaque compte ne consulte que ses propres opérations.
    let chemin = format!("{OPERATIONS}/{A}");
    let consulter = |cookie| request(&banc.router, Method::GET, &chemin, cookie, Value::Null);
    assert_eq!(consulter(Some(&banc.cookie)).await.status, StatusCode::OK);
    assert_eq!(consulter(Some(&bob)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(consulter(None).await.status, StatusCode::UNAUTHORIZED);
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn un_refus_de_la_messagerie_retablit_la_lettre() -> Result<(), Error> {
    let (banc, listener) = banc().await?;
    let mut commande_refusee = selection("SELECT", 1);
    commande_refusee.push((
        "UID STORE 3 -FLAGS (\\Flagged)".into(),
        "{tag} NO refus\r\n".into(),
    ));
    let mut dossier_refuse = login_dialog("OK bienvenue");
    dossier_refuse.push((
        r#"SELECT "INBOX""#.into(),
        "{tag} NO introuvable\r\n".into(),
    ));
    let autre_generation = selection("SELECT", 9);
    let (pair, _suivi) = messagerie(
        listener,
        vec![commande_refusee, dossier_refuse, autre_generation],
    );
    for (id, genre) in [(A, "sans_etoile"), (B, "lu"), (C, "lu")] {
        let reponse = banc.envoyer(OPERATIONS, operation(id, genre)).await;
        assert_eq!(reponse.body["etat"], "en_attente", "{id}");
        let refusee = banc.conclue(id).await;
        assert_eq!(refusee["etat"], "refusee", "{id}");
        assert_eq!(
            refusee["resultats"],
            json!([{"id": LETTRE, "etat": "refuse"}])
        );
        // La projection a porté l’intention, puis le refus a rendu l’état d’avant.
        assert_ne!(refusee["revision"], reponse.body["revision"]);
        assert_eq!(banc.drapeaux()?, ETOILEE);
    }
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn lu_puis_non_lu_rapproches_aboutissent_a_non_lu() -> Result<(), Error> {
    let (banc, listener) = banc().await?;
    let (pair, _suivi) = messagerie(
        listener,
        vec![
            depot(
                "UID STORE 3 +FLAGS (\\Seen)",
                "* 3 FETCH (UID 3 FLAGS (\\Seen \\Flagged))\r\n",
            ),
            depot(
                "UID STORE 3 -FLAGS (\\Seen)",
                "* 3 FETCH (UID 3 FLAGS (\\Flagged))\r\n",
            ),
        ],
    );
    let lue = banc.envoyer(OPERATIONS, operation(A, "lu")).await;
    let non_lue = banc.envoyer(OPERATIONS, operation(B, "non_lu")).await;
    assert_eq!(lue.status, StatusCode::OK);
    assert_eq!(non_lue.status, StatusCode::OK);
    for id in [A, B] {
        assert_eq!(banc.conclue(id).await["etat"], "confirmee", "{id}");
    }
    assert_eq!(banc.drapeaux()?, ETOILEE);
    let page = banc.lire("/api/v1/views/reception").await;
    assert_eq!(page["nouveaux"][0]["id"], "1/fil");
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn une_panne_garde_l_operation_en_attente_et_l_ouverture_passe_apres_elle()
-> Result<(), Error> {
    let (banc, listener) = banc().await?;
    let (pair, mut suivi) = messagerie(
        listener,
        vec![
            hors_service(),
            depot(
                "UID STORE 3 -FLAGS (\\Seen)",
                "* 3 FETCH (UID 3 FLAGS (\\Flagged))\r\n",
            ),
            depot(
                "UID STORE 3 +FLAGS (\\Seen)",
                "* 3 FETCH (UID 3 FLAGS (\\Seen \\Flagged))\r\n",
            ),
            hors_service(),
            hors_service(),
            depot(
                "UID STORE 3 -FLAGS (\\Flagged)",
                "* 3 FETCH (UID 3 FLAGS (\\Seen))\r\n",
            ),
            depot(
                "UID STORE 3 -FLAGS (\\Seen)",
                "* 3 FETCH (UID 3 FLAGS ())\r\n",
            ),
        ],
    );
    // La messagerie est injoignable : l’opération attend.
    banc.envoyer(OPERATIONS, operation(A, "non_lu")).await;
    suivi.recv().await;
    assert_eq!(banc.etat(A).await["etat"], "en_attente");
    // L’ouverture applique d’abord l’opération plus ancienne, puis marque la lettre lue.
    let ouverte = banc.envoyer(OUVERTURE, Value::Null).await;
    assert_eq!(ouverte.status, StatusCode::OK);
    assert_eq!(ouverte.body["non_lus"]["reception"], false);
    assert_eq!(banc.etat(A).await["etat"], "confirmee");
    assert_eq!(banc.drapeaux()?, r#"["\\Flagged","\\Seen"]"#);
    suivi.recv().await;
    suivi.recv().await;
    // Nouvelle panne : ni l’opération ni l’ouverture qui la suit ne sont confirmées.
    banc.envoyer(OPERATIONS, operation(B, "sans_etoile")).await;
    suivi.recv().await;
    let indisponible = banc.envoyer(OUVERTURE, Value::Null).await;
    assert_eq!(indisponible.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(indisponible.body["code"], "lecture_non_confirmee");
    assert_eq!(banc.etat(B).await["etat"], "en_attente");
    assert_eq!(banc.drapeaux()?, r#"["\\Seen"]"#);
    // Au réveil suivant, les opérations en attente passent dans leur ordre d’arrivée.
    banc.envoyer(OPERATIONS, operation(C, "non_lu")).await;
    for id in [B, C] {
        assert_eq!(banc.conclue(id).await["etat"], "confirmee", "{id}");
    }
    assert_eq!(banc.drapeaux()?, "[]");
    sans_autre_commande(pair).await
}
