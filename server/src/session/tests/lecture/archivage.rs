use tokio::net::TcpListener;

use super::{
    operations::{Banc, OPERATIONS, banc, messagerie, sans_autre_commande},
    *,
};

const A: &str = "0190a1b2-0000-7000-8000-0000000000a1";
const B: &str = "0190a1b2-0000-7000-8000-0000000000a2";
const C: &str = "0190a1b2-0000-7000-8000-0000000000a3";
const D: &str = "0190a1b2-0000-7000-8000-0000000000a4";
pub(super) const RECEPTION: &str = "/api/v1/views/reception";
const DOSSIERS: &str = "/api/v1/folders";
const CHOIX: &str = "/api/v1/folders/archive";
const INBOX: i64 = 1;
const ATELIER: i64 = 2;
const INBOX_DE_BOB: i64 = 3;
const ARCHIVES: i64 = 4;
const INCONNU: i64 = 99;
pub(super) const LETTRES: &str =
    "INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,subject,
    received_at,recipients,flags,size,has_attachments,snippet) VALUES
    (1,1,1,5,'1/c','Devis',103,'{}','[]',5,0,''),
    (1,1,1,6,'1/a','À lire',104,'{}','[]',5,0,'');";
const ARCHIVES_DECOUVERTES: &str =
    "INSERT INTO folders(id,account_id,remote_path,role,uidvalidity,uidnext)
    VALUES (4,1,'Archives','archive',7,1);";

// Réception d’Alice : les fils A, C et B (lettre 3, étoilée) du plus récent au plus ancien.
async fn classeur() -> Result<(Banc, TcpListener), Error> {
    let (banc, listener) = banc().await?;
    let db = banc.app.db.writer()?;
    db.execute_batch(LETTRES)?;
    db.execute_batch(ARCHIVES_DECOUVERTES)?;
    drop(db);
    Ok((banc, listener))
}

fn archivage(id: &str, fils: &[&str]) -> Value {
    json!({"id": id, "type": "archiver", "vue": "reception", "cibles": fils})
}

pub(super) fn presentes(avant: &str, trouvees: &str) -> (String, String) {
    ok(
        &format!("UID SEARCH UID {avant}"),
        &format!("* SEARCH{trouvees}\r\n"),
    )
}

pub(super) fn capacites(liste: &str) -> (String, String) {
    ok("CAPABILITY", &format!("* CAPABILITY IMAP4rev1{liste}\r\n"))
}

// Dialogue d’un lot : dossier ouvert, lettres trouvées, capacités, puis les commandes données.
pub(super) fn depart(avant: &str, capacite: &str, commandes: Vec<(String, String)>) -> Dialogue {
    let mut dialogue = selection("SELECT", 1);
    dialogue.push(presentes(avant, &format!(" {}", avant.replace(',', " "))));
    dialogue.push(capacites(capacite));
    dialogue.extend(commandes);
    dialogue
}

pub(super) fn ids(fils: &Value) -> Vec<&str> {
    fils.as_array()
        .unwrap()
        .iter()
        .map(|fil| fil["id"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn archive_la_selection_des_le_geste_puis_rend_la_lettre_refusee() -> Result<(), Error> {
    let (banc, listener) = classeur().await?;
    let mut lecture = selection("SELECT", 1);
    lecture.push(ok(
        "UID STORE 6 +FLAGS (\\Seen)",
        "* 3 FETCH (UID 6 FLAGS (\\Seen))\r\n",
    ));
    let deplacement = depart(
        "3,5",
        " UIDPLUS move",
        vec![
            ok(
                "UID MOVE 3,5 \"Archives\"",
                "* OK [COPYUID 7 3 11] déplacée\r\n* 1 EXPUNGE\r\n",
            ),
            presentes("3,5", " 5"),
        ],
    );
    let (pair, _suivi) = messagerie(listener, vec![lecture, deplacement]);
    // Lire le fil A le marque lu : aucune commande ne le déplace.
    let ouverte = banc
        .envoyer("/api/v1/messages/1-1-6/open", Value::Null)
        .await;
    assert_eq!(ouverte.status, StatusCode::OK);

    let demande = archivage(A, &["1/fil", "1/c"]);
    let reponse = banc.envoyer(OPERATIONS, demande.clone()).await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(reponse.body["type"], "archiver");
    assert_eq!(reponse.body["etat"], "en_attente");
    // La messagerie n’a pas encore répondu : B et C ont déjà quitté la liste.
    let page = banc.lire(RECEPTION).await;
    assert!(ids(&page["nouveaux"]).is_empty());
    assert_eq!(ids(&page["deja_vus"]), ["1/a"]);
    assert_eq!(page["compteurs"]["reception"], 1);
    assert_eq!(page["revision"], reponse.body["revision"]);

    let conclue = banc.conclue(A).await;
    assert_eq!(conclue["etat"], "confirmee");
    assert_eq!(
        conclue["resultats"],
        json!([{"id": "1/fil", "etat": "confirme"}, {"id": "1/c", "etat": "refuse"}])
    );
    // B est rangé dans Archives sous son nouvel UID, avec son étoile. C est revenu.
    let page = banc.lire(RECEPTION).await;
    assert_eq!(ids(&page["nouveaux"]), ["1/c"]);
    assert_eq!(ids(&page["deja_vus"]), ["1/a"]);
    assert_eq!(page["compteurs"]["reception"], 2);
    let archives = banc.lire("/api/v1/folders/4/messages").await;
    assert_eq!(ids(&archives["nouveaux"]), ["1/fil"]);
    assert_eq!(archives["nouveaux"][0]["etoile"], true);
    let fil = banc.lire("/api/v1/threads/1%2Ffil").await;
    assert_eq!(fil[0]["id"], "4-7-11");
    assert_eq!(banc.lire("/api/v1/threads/1%2Fc").await[0]["id"], "1-1-5");

    // Le rejeu rend le résultat connu : aucun second déplacement.
    let rejeu = banc.envoyer(OPERATIONS, demande).await;
    assert_eq!(rejeu.status, StatusCode::OK);
    assert_eq!(rejeu.body, conclue);
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn sans_move_copie_puis_efface_les_seules_lettres_archivees() -> Result<(), Error> {
    let (banc, listener) = classeur().await?;
    let copie = depart(
        "3",
        " UIDPLUS",
        vec![
            (
                "UID COPY 3 \"Archives\"".into(),
                "{tag} OK [COPYUID 7 3 12] copiée\r\n".into(),
            ),
            ok("UID STORE 3 +FLAGS.SILENT (\\Deleted)", ""),
            ok("UID EXPUNGE 3", "* 1 EXPUNGE\r\n"),
            presentes("3", ""),
        ],
    );
    let (pair, _suivi) = messagerie(listener, vec![copie]);
    banc.envoyer(OPERATIONS, archivage(A, &["1/fil"])).await;
    let conclue = banc.conclue(A).await;
    assert_eq!(conclue["etat"], "confirmee");
    assert_eq!(
        conclue["resultats"],
        json!([{"id": "1/fil", "etat": "confirme"}])
    );
    let fil = banc.lire("/api/v1/threads/1%2Ffil").await;
    assert_eq!(fil[0]["id"], "4-7-12");
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn un_refus_de_la_messagerie_rend_le_fil_a_reception() -> Result<(), Error> {
    let (banc, listener) = classeur().await?;
    let refus = |commande: &str| (commande.to_owned(), "{tag} NO refus\r\n".to_owned());
    let reste = || presentes("3", " 3");
    let mut dossier_ferme = login_dialog("OK bienvenue");
    dossier_ferme.push(refus(r#"SELECT "INBOX""#));
    let (pair, _suivi) = messagerie(
        listener,
        vec![
            // Ni MOVE ni effacement ciblé : rien n’est tenté.
            depart("3", "", vec![reste()]),
            depart(
                "3",
                " MOVE",
                vec![refus("UID MOVE 3 \"Archives\""), reste()],
            ),
            depart(
                "3",
                " UIDPLUS",
                vec![refus("UID COPY 3 \"Archives\""), reste()],
            ),
            dossier_ferme,
        ],
    );
    for id in [A, B, C, D] {
        let reponse = banc.envoyer(OPERATIONS, archivage(id, &["1/fil"])).await;
        assert_eq!(reponse.body["etat"], "en_attente", "{id}");
        let refusee = banc.conclue(id).await;
        assert_eq!(refusee["etat"], "refusee", "{id}");
        assert_eq!(
            refusee["resultats"],
            json!([{"id": "1/fil", "etat": "refuse"}])
        );
        assert_ne!(refusee["revision"], reponse.body["revision"]);
        let page = banc.lire(RECEPTION).await;
        assert_eq!(ids(&page["nouveaux"]), ["1/a", "1/c", "1/fil"], "{id}");
        assert_eq!(page["compteurs"]["reception"], 3);
    }
    let fil = banc.lire("/api/v1/threads/1%2Ffil").await;
    assert_eq!(fil[0]["id"], "1-1-3");
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn une_coupure_apres_l_envoi_ne_fait_pas_passer_la_lettre_pour_disparue() -> Result<(), Error>
{
    let (banc, listener) = classeur().await?;
    // La connexion se ferme sans réponse à la commande de déplacement.
    let coupee = depart(
        "3,5",
        " MOVE",
        vec![("UID MOVE 3,5 \"Archives\"".into(), String::new())],
    );
    let mut reprise = selection("SELECT", 1);
    reprise.extend([
        presentes("3,5", " 5"),
        capacites(" MOVE"),
        ok("UID MOVE 5 \"Archives\"", ""),
        presentes("5", ""),
    ]);
    let (pair, mut suivi) = messagerie(listener, vec![coupee, reprise]);
    let demande = archivage(A, &["1/fil", "1/c"]);
    banc.envoyer(OPERATIONS, demande.clone()).await;
    suivi.recv().await;
    assert_eq!(banc.etat(A).await["etat"], "en_attente");
    // Le rejeu réveille l’application : la lettre 3 est partie avec la première commande.
    banc.envoyer(OPERATIONS, demande).await;
    let conclue = banc.conclue(A).await;
    assert_eq!(conclue["etat"], "confirmee");
    assert_eq!(
        conclue["resultats"],
        json!([{"id": "1/fil", "etat": "confirme"}, {"id": "1/c", "etat": "confirme"}])
    );
    // Sans nouvel UID annoncé, les lettres attendent l’import de leur dossier d’arrivée.
    let page = banc.lire(RECEPTION).await;
    assert_eq!(ids(&page["nouveaux"]), ["1/a"]);
    let archives = banc.lire("/api/v1/folders/4/messages").await;
    assert!(ids(&archives["nouveaux"]).is_empty());
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn sans_dossier_d_archives_demande_puis_retient_la_destination() -> Result<(), Error> {
    let (banc, listener) = banc().await?;
    let vers_atelier = depart(
        "3",
        " MOVE",
        vec![
            ok(
                "UID MOVE 3 \"Atelier\"",
                "* OK [COPYUID 1 3 8] déplacée\r\n",
            ),
            presentes("3", ""),
        ],
    );
    let (pair, _suivi) = messagerie(listener, vec![vers_atelier]);
    let sans_arrivee = banc.envoyer(OPERATIONS, archivage(A, &["1/fil"])).await;
    assert_eq!(sans_arrivee.status, StatusCode::CONFLICT);
    assert_eq!(sans_arrivee.body["code"], "archives_inconnues");
    assert_eq!(banc.etat(A).await["code"], "operation_introuvable");
    let page = banc.lire(RECEPTION).await;
    assert_eq!(ids(&page["nouveaux"]), ["1/fil"]);

    let choisir = |cookie, corps| request(&banc.router, Method::PUT, CHOIX, cookie, corps);
    let sans_session = choisir(None, json!({"dossier": ATELIER})).await;
    assert_eq!(sans_session.status, StatusCode::UNAUTHORIZED);
    // Seul un dossier personnel ouvrable de ce compte peut recevoir les archives.
    banc.app.db.writer()?.execute_batch(
        "INSERT INTO folders(id,account_id,remote_path) VALUES (5,1,'Classement');",
    )?;
    for dossier in [INBOX, INBOX_DE_BOB, 5, INCONNU] {
        let refuse = choisir(Some(&banc.cookie), json!({"dossier": dossier})).await;
        assert_eq!(refuse.status, StatusCode::UNPROCESSABLE_ENTITY, "{dossier}");
        assert_eq!(refuse.body["code"], "dossier_invalide");
    }
    let illisible = choisir(Some(&banc.cookie), json!({"dossier": "Atelier"})).await;
    assert_eq!(illisible.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(illisible.body["code"], "dossier_invalide");
    let avant = banc.lire(DOSSIERS).await;
    assert_eq!(
        avant["folders"],
        json!([
            {"id": ATELIER, "path": "Atelier", "delimiter": null, "role": null,
             "hidden": false, "selectable": true},
            {"id": 5, "path": "Classement", "delimiter": null, "role": null,
             "hidden": false, "selectable": false},
            {"id": INBOX, "path": "INBOX", "delimiter": null, "role": "inbox",
             "hidden": false, "selectable": true},
        ])
    );

    let choisi = choisir(Some(&banc.cookie), json!({"dossier": ATELIER})).await;
    assert_eq!(choisi.status, StatusCode::OK);
    assert_eq!(choisi.body["folders"][0]["role"], "archive");
    assert_eq!(choisi.body, banc.lire(DOSSIERS).await);
    // Le choix est retenu pour le compte : l’archivage suivant ne demande plus rien.
    let reponse = banc.envoyer(OPERATIONS, archivage(A, &["1/fil"])).await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(banc.conclue(A).await["etat"], "confirmee");
    let fil = banc.lire("/api/v1/threads/1%2Ffil").await;
    assert_eq!(fil[0]["id"], "2-1-8");

    // Découvert plus tard, un dossier d’archives ne remplace pas le choix de l’utilisateur.
    banc.app.db.writer()?.execute_batch(ARCHIVES_DECOUVERTES)?;
    let roles: Vec<Value> = banc.lire(DOSSIERS).await["folders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|dossier| json!([dossier["path"], dossier["role"]]))
        .collect();
    assert_eq!(
        json!(roles),
        json!([
            ["Archives", null],
            ["Atelier", "archive"],
            ["Classement", null],
            ["INBOX", "inbox"]
        ])
    );
    let autre = choisir(Some(&banc.cookie), json!({"dossier": ARCHIVES})).await;
    assert_eq!(autre.status, StatusCode::UNPROCESSABLE_ENTITY);
    sans_autre_commande(pair).await
}
