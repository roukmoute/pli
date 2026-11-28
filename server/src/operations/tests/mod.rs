use std::collections::BTreeMap;

use serde_json::{Value, json};

mod archivage;
mod corbeille;

use super::stockage::{
    Action, Demande, Depot, Genre, Lot, comptes_en_attente, conclure, creer, lire, prochain,
};
use crate::{
    Error,
    test_support::{BOITES_DE_TEST, TestApp},
};

const A: &str = "0190a1b2-0000-7000-8000-00000000000a";
const B: &str = "0190a1b2-0000-7000-8000-00000000000b";
const C: &str = "0190a1b2-0000-7000-8000-00000000000c";
const LETTRE: &str = "1-1-3";
const ALICE: i64 = 1;
const BOB: i64 = 2;
const INBOX: i64 = 1;
const ATELIER: i64 = 2;
const INBOX_DE_BOB: i64 = 3;

// Le fil d’Alice réunit deux lettres de Réception, dont une déjà lue, et une lettre d’Atelier.
fn boite() -> Result<TestApp, Error> {
    let app = TestApp::new()?;
    {
        let db = app.db.writer()?;
        db.execute_batch(BOITES_DE_TEST)?;
        db.execute_batch(
            r#"UPDATE inbox_markers SET uidnext=9 WHERE account_id=2;
            INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,subject,
                received_at,recipients,flags,size,has_attachments,snippet) VALUES
                (1,1,1,3,'1/fil','Plan',100,'{}','[]',5,0,''),
                (1,1,1,4,'1/fil','Re: Plan',101,'{}','["\\Seen"]',5,0,''),
                (1,2,1,7,'1/fil','Copie',102,'{}','[]',5,0,''),
                (2,3,1,3,'2/prive','Carnet',100,'{}','[]',5,0,'');"#,
        )?;
    }
    Ok(app)
}

fn demande(id: &str, genre: Genre, vue: Option<&str>, cibles: &[&str]) -> Demande {
    Demande {
        id: id.into(),
        genre,
        vue: vue.map(str::to_owned),
        cibles: cibles.iter().map(|&c| c.to_owned()).collect(),
        destination: None,
    }
}

// Rend l’opération enregistrée, ou le nom de ce qui a empêché de l’enregistrer.
fn enregistrer(app: &TestApp, compte: i64, demande: &Demande) -> Result<Value, Error> {
    Ok(match creer(&app.db, compte, demande)? {
        Ok(operation) => serde_json::to_value(operation)?,
        Err(empechement) => json!(format!("{empechement:?}")),
    })
}

fn connue(app: &TestApp, compte: i64, id: &str) -> Result<Value, Error> {
    Ok(serde_json::to_value(lire(&*app.db.reader()?, compte, id)?)?)
}

fn drapeaux(app: &TestApp, dossier: i64, uid: u32) -> Result<String, Error> {
    Ok(app.db.reader()?.query_row(
        "SELECT flags FROM messages WHERE (folder_id,uid)=(?1,?2)",
        rusqlite::params![dossier, uid],
        |r| r.get(0),
    )?)
}

fn revision(app: &TestApp, compte: i64) -> Result<i64, Error> {
    Ok(app.db.reader()?.query_row(
        "SELECT revision FROM account_revisions WHERE account_id=?1",
        [compte],
        |r| r.get(0),
    )?)
}

fn resultats(operation: &Value) -> Vec<(&str, &str)> {
    operation["resultats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["id"].as_str().unwrap(), r["etat"].as_str().unwrap()))
        .collect()
}

// Applique le prochain lot comme le ferait la messagerie avec cette réponse.
fn deposer(app: &TestApp, compte: i64, depot: Depot) -> Result<Vec<u32>, Error> {
    let lot = prochain(&app.db, compte)?.ok_or("Aucun lot en attente.")?;
    conclure(&app.db, compte, &lot, &depot)?;
    Ok(lot.uids)
}

fn confirmes(uids: &[u32]) -> Depot {
    Depot::Applique(uids.iter().map(|&uid| (uid, true)).collect())
}

fn drapeau(lot: &Lot) -> (&str, bool) {
    match &lot.action {
        Action::Drapeau { drapeau, valeur } => (drapeau, *valeur),
        _ => panic!("Lot de drapeau attendu."),
    }
}

#[test]
fn une_operation_projette_son_etat_et_se_rejoue_sans_rien_reappliquer() -> Result<(), Error> {
    let app = boite()?;
    let avant = revision(&app, ALICE)?;
    let lue = demande(A, Genre::Lu, Some("reception"), &["1/fil", "2/prive"]);
    let operation = enregistrer(&app, ALICE, &lue)?;
    assert_eq!(operation["id"], A);
    assert_eq!(operation["type"], "lu");
    assert_eq!(operation["etat"], "en_attente");
    assert_eq!(
        resultats(&operation),
        [("1/fil", "en_attente"), ("2/prive", "absent")]
    );
    // La projection et la révision changent dans la même écriture.
    assert_eq!(drapeaux(&app, INBOX, 3)?, r#"["\\Seen"]"#);
    assert_eq!(drapeaux(&app, INBOX, 4)?, r#"["\\Seen"]"#);
    assert_eq!(drapeaux(&app, ATELIER, 7)?, "[]");
    assert_eq!(drapeaux(&app, INBOX_DE_BOB, 3)?, "[]");
    let apres = revision(&app, ALICE)?;
    assert_ne!(apres, avant);
    assert_eq!(operation["revision"], apres);

    let rejeu = enregistrer(&app, ALICE, &demande(A, Genre::NonLu, None, &[LETTRE]))?;
    assert_eq!(rejeu, operation);
    assert_eq!(connue(&app, ALICE, A)?, operation);
    assert_eq!(revision(&app, ALICE)?, apres);
    assert_eq!(drapeaux(&app, INBOX, 3)?, r#"["\\Seen"]"#);
    // L’identifiant appartient au compte qui l’a choisi.
    assert_eq!(connue(&app, BOB, A)?, Value::Null);
    assert_eq!(connue(&app, ALICE, B)?, Value::Null);
    let autre = enregistrer(&app, BOB, &demande(A, Genre::Etoile, None, &["3-1-3"]))?;
    assert_eq!(autre["type"], "etoile");
    assert_eq!(drapeaux(&app, INBOX_DE_BOB, 3)?, r#"["\\Flagged"]"#);
    assert_eq!(connue(&app, ALICE, A)?, operation);
    Ok(())
}

#[test]
fn une_cible_etrangere_ou_illisible_refuse_l_operation_sans_rien_toucher() -> Result<(), Error> {
    let app = boite()?;
    let avant = revision(&app, ALICE)?;
    let cibles = ["3-1-3", "2/prive", "1-9-3", "inconnu"];
    let refusee = enregistrer(&app, ALICE, &demande(A, Genre::Lu, None, &cibles))?;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(resultats(&refusee), cibles.map(|cible| (cible, "absent")));
    assert_eq!(drapeaux(&app, INBOX_DE_BOB, 3)?, "[]");
    assert_eq!(revision(&app, ALICE)?, avant);
    assert!(prochain(&app.db, ALICE)?.is_none());
    assert!(comptes_en_attente(&app.db)?.is_empty());
    // Un fil se désigne dans une vue : sans elle, son identifiant ne vise aucune lettre.
    let sans_vue = enregistrer(&app, ALICE, &demande(B, Genre::Lu, None, &["1/fil"]))?;
    assert_eq!(sans_vue["etat"], "refusee");
    let ailleurs = enregistrer(
        &app,
        ALICE,
        &demande(C, Genre::Lu, Some("flux"), &["1/fil"]),
    )?;
    assert_eq!(resultats(&ailleurs), [("1/fil", "absent")]);
    Ok(())
}

#[test]
fn chaque_dossier_forme_un_lot_et_la_derniere_lettre_clot_l_operation() -> Result<(), Error> {
    let app = boite()?;
    let cibles = [LETTRE, "2-1-7", LETTRE, "1-1-4"];
    enregistrer(&app, ALICE, &demande(A, Genre::Etoile, None, &cibles))?;
    assert_eq!(comptes_en_attente(&app.db)?, [ALICE]);
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.chemin.as_deref(), Some("INBOX"));
    assert_eq!(lot.generation, 1);
    assert_eq!(drapeau(&lot), ("\\Flagged", true));
    assert_eq!(lot.uids, [3, 4]);
    // La lettre 4 n’est pas rendue par le serveur, la lettre 3 est confirmée.
    conclure(&app.db, ALICE, &lot, &confirmes(&[3]))?;
    let partielle = connue(&app, ALICE, A)?;
    assert_eq!(partielle["etat"], "en_attente");
    assert_eq!(
        resultats(&partielle),
        [
            (LETTRE, "confirme"),
            ("2-1-7", "en_attente"),
            (LETTRE, "confirme"),
            ("1-1-4", "absent")
        ]
    );
    assert_eq!(drapeaux(&app, INBOX, 3)?, r#"["\\Flagged"]"#);
    assert_eq!(drapeaux(&app, INBOX, 4)?, r#"["\\Seen"]"#);
    assert_eq!(drapeaux(&app, ATELIER, 7)?, r#"["\\Flagged"]"#);
    let atelier = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(atelier.chemin.as_deref(), Some("Atelier"));
    assert_eq!(atelier.uids, [7]);
    // Le serveur rend la lettre sans le drapeau demandé.
    let sans_drapeau = Depot::Applique(BTreeMap::from([(7, false)]));
    conclure(&app.db, ALICE, &atelier, &sans_drapeau)?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(resultats(&close)[1], ("2-1-7", "refuse"));
    assert_eq!(drapeaux(&app, ATELIER, 7)?, "[]");
    assert!(prochain(&app.db, ALICE)?.is_none());
    assert!(comptes_en_attente(&app.db)?.is_empty());
    Ok(())
}

#[test]
fn un_fil_partiellement_confirme_garde_les_lettres_reussies() -> Result<(), Error> {
    let app = boite()?;
    enregistrer(
        &app,
        ALICE,
        &demande(A, Genre::NonLu, Some("reception"), &["1/fil"]),
    )?;
    assert_eq!(drapeaux(&app, INBOX, 4)?, "[]");
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(drapeau(&lot), ("\\Seen", false));
    conclure(&app.db, ALICE, &lot, &confirmes(&[3]))?;
    let operation = connue(&app, ALICE, A)?;
    assert_eq!(operation["etat"], "confirmee");
    assert_eq!(resultats(&operation), [("1/fil", "partiel")]);
    // Seule la lettre non confirmée retrouve son état d’avant.
    assert_eq!(drapeaux(&app, INBOX, 3)?, "[]");
    assert_eq!(drapeaux(&app, INBOX, 4)?, r#"["\\Seen"]"#);
    Ok(())
}

#[test]
fn un_refus_retablit_la_projection_et_avance_la_revision() -> Result<(), Error> {
    let app = boite()?;
    let operation = enregistrer(&app, ALICE, &demande(A, Genre::Etoile, None, &[LETTRE]))?;
    assert_eq!(drapeaux(&app, INBOX, 3)?, r#"["\\Flagged"]"#);
    assert_eq!(deposer(&app, ALICE, Depot::Refuse)?, [3]);
    let refusee = connue(&app, ALICE, A)?;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(resultats(&refusee), [(LETTRE, "refuse")]);
    assert_eq!(drapeaux(&app, INBOX, 3)?, "[]");
    assert_ne!(refusee["revision"], operation["revision"]);
    // Retirer une étoile refusée la rend.
    enregistrer(&app, BOB, &demande(A, Genre::Etoile, None, &["3-1-3"]))?;
    deposer(&app, BOB, confirmes(&[3]))?;
    enregistrer(&app, BOB, &demande(B, Genre::SansEtoile, None, &["3-1-3"]))?;
    assert_eq!(drapeaux(&app, INBOX_DE_BOB, 3)?, "[]");
    deposer(&app, BOB, Depot::Refuse)?;
    assert_eq!(drapeaux(&app, INBOX_DE_BOB, 3)?, r#"["\\Flagged"]"#);
    Ok(())
}

#[test]
fn un_refus_laisse_la_main_a_l_intention_suivante_sur_le_meme_drapeau() -> Result<(), Error> {
    let app = boite()?;
    enregistrer(&app, ALICE, &demande(A, Genre::Lu, None, &[LETTRE]))?;
    enregistrer(&app, ALICE, &demande(B, Genre::Etoile, None, &[LETTRE]))?;
    enregistrer(&app, ALICE, &demande(C, Genre::Lu, None, &[LETTRE]))?;
    assert_eq!(drapeaux(&app, INBOX, 3)?, r#"["\\Seen","\\Flagged"]"#);
    // Le premier « lu » est refusé : le second, plus récent, garde la lettre lue.
    deposer(&app, ALICE, Depot::Refuse)?;
    assert_eq!(connue(&app, ALICE, A)?["etat"], "refusee");
    assert_eq!(drapeaux(&app, INBOX, 3)?, r#"["\\Seen","\\Flagged"]"#);
    // L’étoile refusée n’a pas d’intention plus récente : elle disparaît.
    deposer(&app, ALICE, Depot::Refuse)?;
    assert_eq!(drapeaux(&app, INBOX, 3)?, r#"["\\Seen"]"#);
    // Le second « lu » hérite de l’état d’avant le premier : la lettre redevient non lue.
    deposer(&app, ALICE, Depot::Refuse)?;
    assert_eq!(connue(&app, ALICE, C)?["etat"], "refusee");
    assert_eq!(drapeaux(&app, INBOX, 3)?, "[]");
    Ok(())
}

#[test]
fn deux_intentions_opposees_convergent_vers_la_derniere() -> Result<(), Error> {
    let app = boite()?;
    enregistrer(&app, ALICE, &demande(A, Genre::Lu, None, &[LETTRE]))?;
    enregistrer(&app, ALICE, &demande(B, Genre::NonLu, None, &[LETTRE]))?;
    assert_eq!(drapeaux(&app, INBOX, 3)?, "[]");
    let lue = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(drapeau(&lue), ("\\Seen", true));
    conclure(&app.db, ALICE, &lue, &confirmes(&[3]))?;
    let non_lue = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(drapeau(&non_lue), ("\\Seen", false));
    conclure(&app.db, ALICE, &non_lue, &confirmes(&[3]))?;
    for id in [A, B] {
        assert_eq!(connue(&app, ALICE, id)?["etat"], "confirmee");
    }
    assert_eq!(drapeaux(&app, INBOX, 3)?, "[]");
    Ok(())
}

#[test]
fn un_dossier_regenere_rend_ses_lettres_absentes() -> Result<(), Error> {
    let app = boite()?;
    enregistrer(&app, ALICE, &demande(A, Genre::Lu, None, &["2-1-7"]))?;
    app.db
        .writer()?
        .execute("UPDATE folders SET uidvalidity=2 WHERE id=2", [])?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.chemin, None);
    assert_eq!(lot.uids, [7]);
    conclure(&app.db, ALICE, &lot, &Depot::Applique(BTreeMap::new()))?;
    let operation = connue(&app, ALICE, A)?;
    assert_eq!(operation["etat"], "refusee");
    assert_eq!(resultats(&operation), [("2-1-7", "absent")]);
    assert_eq!(drapeaux(&app, ATELIER, 7)?, "[]");
    Ok(())
}

#[test]
fn un_lot_ne_depasse_pas_cent_lettres() -> Result<(), Error> {
    let app = boite()?;
    app.db.writer()?.execute_batch(
        "WITH RECURSIVE suite(uid) AS (SELECT 100 UNION ALL SELECT uid+1 FROM suite WHERE uid<200)
         INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,subject,
             received_at,recipients,flags,size,has_attachments,snippet)
         SELECT 1,2,1,uid,'1/long','Carnet',uid,'{}','[]',5,0,'' FROM suite;",
    )?;
    let operation = enregistrer(
        &app,
        ALICE,
        &demande(A, Genre::Lu, Some("dossier/2"), &["1/long"]),
    )?;
    assert_eq!(
        operation,
        json!({
            "id": A, "type": "lu", "etat": "en_attente", "revision": revision(&app, ALICE)?,
            "resultats": [{"id": "1/long", "etat": "en_attente"}]
        })
    );
    let premier = deposer(&app, ALICE, Depot::Refuse)?;
    assert_eq!(premier.len(), 100);
    assert_eq!(premier.first().zip(premier.last()), Some((&100, &199)));
    assert_eq!(connue(&app, ALICE, A)?["etat"], "en_attente");
    assert_eq!(deposer(&app, ALICE, confirmes(&[200]))?, [200]);
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(resultats(&close), [("1/long", "partiel")]);
    assert_eq!(drapeaux(&app, ATELIER, 150)?, "[]");
    assert_eq!(drapeaux(&app, ATELIER, 200)?, r#"["\\Seen"]"#);
    Ok(())
}
