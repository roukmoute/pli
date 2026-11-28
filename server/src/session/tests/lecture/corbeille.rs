use tokio::net::TcpListener;

use super::{
    archivage::{LETTRES, RECEPTION, capacites, depart, ids, presentes},
    operations::{Banc, OPERATIONS, banc, messagerie, sans_autre_commande},
    *,
};

const A: &str = "0190a1b2-0000-7000-8000-0000000000b1";
const B: &str = "0190a1b2-0000-7000-8000-0000000000b2";
const C: &str = "0190a1b2-0000-7000-8000-0000000000b3";
const CORBEILLE: &str = "/api/v1/folders/5/messages";
const JETEES: &str = "dossier/5";
const ATELIER: i64 = 2;
const DOSSIERS: &str =
    "INSERT INTO folders(id,account_id,remote_path,role,uidvalidity,uidnext) VALUES
    (5,1,'Papierkorb','trash',9,1);";
// Trois lettres jetées depuis une autre messagerie : leur dossier d’origine est inconnu.
const JETEES_AILLEURS: &str = "INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,
    subject,received_at,recipients,flags,size,has_attachments,snippet) VALUES
    (1,5,9,41,'1/x','Brouillon',200,'{}','[]',5,0,''),
    (1,5,9,42,'1/y','Doublon',201,'{}','[]',5,0,''),
    (1,5,9,43,'1/z','Réclame',202,'{}','[]',5,0,'');";

// Réception d’Alice : les fils A, C et B. Sa messagerie annonce une corbeille.
async fn boite(lettres: &str) -> Result<(Banc, TcpListener), Error> {
    let (banc, listener) = banc().await?;
    let db = banc.app.db.writer()?;
    db.execute_batch(LETTRES)?;
    db.execute_batch(DOSSIERS)?;
    db.execute_batch(lettres)?;
    drop(db);
    Ok((banc, listener))
}

fn demande(id: &str, genre: &str, vue: &str, cibles: &[&str]) -> Value {
    json!({"id": id, "type": genre, "vue": vue, "cibles": cibles})
}

// Dialogue d’un lot parti de la Corbeille : dossier ouvert, lettres trouvées, capacités.
fn depuis_la_corbeille(avant: &str, capacite: &str, commandes: Vec<(String, String)>) -> Dialogue {
    let mut dialogue = login_dialog("OK bienvenue");
    dialogue.push(ok(
        "SELECT \"Papierkorb\"",
        "* OK [UIDVALIDITY 9] valable\r\n",
    ));
    dialogue.push(presentes(avant, &format!(" {}", avant.replace(',', " "))));
    dialogue.push(capacites(capacite));
    dialogue.extend(commandes);
    dialogue
}

fn effacement(lettres: &str) -> Vec<(String, String)> {
    vec![
        ok(
            &format!("UID STORE {lettres} +FLAGS.SILENT (\\Deleted)"),
            "",
        ),
        ok(&format!("UID EXPUNGE {lettres}"), "* 1 EXPUNGE\r\n"),
        presentes(lettres, ""),
    ]
}

#[tokio::test]
async fn supprimer_puis_restaurer_ajuste_la_corbeille_et_la_reception() -> Result<(), Error> {
    let (banc, listener) = boite("").await?;
    let vers_la_corbeille = depart(
        "5",
        " MOVE",
        vec![
            ok(
                "UID MOVE 5 \"Papierkorb\"",
                "* OK [COPYUID 9 5 41] jetée\r\n",
            ),
            presentes("5", ""),
        ],
    );
    let retour = depuis_la_corbeille(
        "41",
        " MOVE",
        vec![
            ok("UID MOVE 41 \"INBOX\"", "* OK [COPYUID 1 41 12] rendue\r\n"),
            presentes("41", ""),
        ],
    );
    let (pair, _suivi) = messagerie(listener, vec![vers_la_corbeille, retour]);
    assert_eq!(banc.lire(RECEPTION).await["total"], 3);
    assert_eq!(banc.lire(CORBEILLE).await["total"], 0);

    // Supprimer ne demande rien : le fil quitte Réception dès la réponse.
    let jetee = banc
        .envoyer(OPERATIONS, demande(A, "corbeille", "reception", &["1/c"]))
        .await;
    assert_eq!(jetee.status, StatusCode::OK);
    assert_eq!(jetee.body["etat"], "en_attente");
    let page = banc.lire(RECEPTION).await;
    assert_eq!(ids(&page["nouveaux"]), ["1/a", "1/fil"]);
    assert_eq!(page["total"], 2);
    assert_eq!(banc.conclue(A).await["etat"], "confirmee");
    let corbeille = banc.lire(CORBEILLE).await;
    assert_eq!(ids(&corbeille["nouveaux"]), ["1/c"]);
    assert_eq!(corbeille["total"], 1);
    // Le fil sait d’où il vient : rien ne signale une origine inconnue.
    assert_eq!(corbeille["nouveaux"][0].get("sans_origine"), None);

    // Restaurer le ramène dans son dossier d’origine et ajuste les deux compteurs.
    let rendue = banc
        .envoyer(OPERATIONS, demande(B, "restaurer", JETEES, &["1/c"]))
        .await;
    assert_eq!(rendue.status, StatusCode::OK);
    assert_eq!(banc.lire(CORBEILLE).await["total"], 0);
    let conclue = banc.conclue(B).await;
    assert_eq!(conclue["etat"], "confirmee");
    assert_eq!(
        conclue["resultats"],
        json!([{"id": "1/c", "etat": "confirme"}])
    );
    let page = banc.lire(RECEPTION).await;
    assert_eq!(ids(&page["nouveaux"]), ["1/a", "1/c", "1/fil"]);
    assert_eq!(page["total"], 3);
    assert_eq!(banc.lire(CORBEILLE).await["total"], 0);
    let fil = banc.lire("/api/v1/threads/1%2Fc").await;
    assert_eq!(fil[0]["id"], "1-1-12");
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn la_suppression_definitive_n_expurge_que_les_lettres_ciblees() -> Result<(), Error> {
    let (banc, listener) = boite(JETEES_AILLEURS).await?;
    let ciblees = depuis_la_corbeille("41,42", " UIDPLUS", effacement("41,42"));
    // Sans effacement ciblé, la messagerie ne reçoit aucune commande de suppression.
    let sans_uidplus = depuis_la_corbeille("43", " MOVE", vec![presentes("43", " 43")]);
    let videe = depuis_la_corbeille("43", " UIDPLUS", effacement("43"));
    let (pair, _suivi) = messagerie(listener, vec![ciblees, sans_uidplus, videe]);
    let corbeille = banc.lire(CORBEILLE).await;
    assert_eq!(corbeille["total"], 3);
    assert_eq!(corbeille["nouveaux"][0]["sans_origine"], true);

    let effacer = demande(A, "supprimer_definitivement", JETEES, &["1/x", "1/y"]);
    let reponse = banc.envoyer(OPERATIONS, effacer).await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(ids(&banc.lire(CORBEILLE).await["nouveaux"]), ["1/z"]);
    let conclue = banc.conclue(A).await;
    assert_eq!(conclue["etat"], "confirmee");
    assert_eq!(
        conclue["resultats"],
        json!([{"id": "1/x", "etat": "confirme"}, {"id": "1/y", "etat": "confirme"}])
    );
    // La lettre que personne n’a ciblée est toujours dans la Corbeille.
    let corbeille = banc.lire(CORBEILLE).await;
    assert_eq!(ids(&corbeille["nouveaux"]), ["1/z"]);
    assert_eq!(corbeille["total"], 1);

    let derniere = demande(B, "supprimer_definitivement", JETEES, &["1/z"]);
    banc.envoyer(OPERATIONS, derniere).await;
    let refusee = banc.conclue(B).await;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(
        refusee["resultats"],
        json!([{"id": "1/z", "etat": "refuse"}])
    );
    assert_eq!(banc.lire(CORBEILLE).await["total"], 1);

    // Vider le dossier efface toutes les lettres qu’il contient encore.
    let vidage = json!({"id": C, "type": "vider_dossier", "cibles": [JETEES]});
    let reponse = banc.envoyer(OPERATIONS, vidage).await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(banc.lire(CORBEILLE).await["total"], 0);
    let conclue = banc.conclue(C).await;
    assert_eq!(conclue["etat"], "confirmee");
    assert_eq!(
        conclue["resultats"],
        json!([{"id": JETEES, "etat": "confirme"}])
    );
    assert_eq!(banc.lire(RECEPTION).await["total"], 3);
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn sans_corbeille_ou_sans_origine_la_demande_dit_ce_qui_manque() -> Result<(), Error> {
    let (banc, listener) = banc().await?;
    let vers_atelier = depuis_la_corbeille(
        "41",
        " MOVE",
        vec![
            ok(
                "UID MOVE 41 \"Atelier\"",
                "* OK [COPYUID 1 41 8] rendue\r\n",
            ),
            presentes("41", ""),
        ],
    );
    let (pair, _suivi) = messagerie(listener, vec![vers_atelier]);
    let jeter = demande(A, "corbeille", "reception", &["1/fil"]);
    let sans_corbeille = banc.envoyer(OPERATIONS, jeter).await;
    assert_eq!(sans_corbeille.status, StatusCode::CONFLICT);
    assert_eq!(sans_corbeille.body["code"], "corbeille_inconnue");
    assert_eq!(banc.etat(A).await["code"], "operation_introuvable");

    let db = banc.app.db.writer()?;
    db.execute_batch(DOSSIERS)?;
    db.execute_batch(JETEES_AILLEURS)?;
    drop(db);
    let restaurer = demande(A, "restaurer", JETEES, &["1/x"]);
    let sans_origine = banc.envoyer(OPERATIONS, restaurer.clone()).await;
    assert_eq!(sans_origine.status, StatusCode::CONFLICT);
    assert_eq!(sans_origine.body["code"], "origine_inconnue");
    assert_eq!(banc.lire(CORBEILLE).await["total"], 3);
    // Le dossier choisi par l’utilisateur remplace l’origine manquante.
    let mut choisie = restaurer;
    choisie["destination"] = json!(ATELIER);
    let reponse = banc.envoyer(OPERATIONS, choisie).await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(banc.conclue(A).await["etat"], "confirmee");
    let fil = banc.lire("/api/v1/threads/1%2Fx").await;
    assert_eq!(fil[0]["id"], "2-1-8");
    sans_autre_commande(pair).await
}
