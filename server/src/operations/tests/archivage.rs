use std::collections::BTreeMap;

use rusqlite::params;
use serde_json::Value;

use super::{
    A, ALICE, ATELIER, B, INBOX, boite, confirmes, connue, demande, deposer, drapeaux, enregistrer,
    resultats, revision,
};
use crate::{
    Error,
    operations::{
        deplacement::tenter,
        stockage::{Action, Depot, Genre, Lot, Sort, conclure, prochain},
    },
    test_support::TestApp,
};

const ARCHIVES: i64 = 4;
const GENERATION: u32 = 7;
const RECEPTION: &str = "reception";
const CLASSEES: &str = "dossier/4";

// Réception d’Alice : le fil A déjà consulté, le fil B de deux lettres et le fil C.
fn classeur() -> Result<TestApp, Error> {
    let app = boite()?;
    app.db.writer()?.execute_batch(
        r#"INSERT INTO folders(id,account_id,remote_path,role,uidvalidity,uidnext)
               VALUES (4,1,'Archives','archive',7,1);
           INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,subject,
               received_at,recipients,flags,size,has_attachments,snippet) VALUES
               (1,1,1,5,'1/c','Devis',103,'{}','[]',5,0,''),
               (1,1,1,6,'1/a','Consulté',104,'{}','["\\Seen"]',5,0,'');"#,
    )?;
    Ok(app)
}

fn archivage(id: &str, cibles: &[&str]) -> super::Demande {
    demande(id, Genre::Archiver, Some(RECEPTION), cibles)
}

fn fils(app: &TestApp, portee: &str) -> Result<Vec<String>, Error> {
    let db = app.db.reader()?;
    let fils = db
        .prepare(
            "SELECT item FROM list_threads WHERE (account_id,scope)=(?1,?2)
             ORDER BY received_at DESC",
        )?
        .query_map(params![ALICE, portee], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(fils)
}

// Emplacement de chaque lettre du fil : dossier, génération, UID et départ en cours.
fn lettres(app: &TestApp, fil: &str) -> Result<Vec<(i64, u32, u32, bool)>, Error> {
    let db = app.db.reader()?;
    let lettres = db
        .prepare(
            "SELECT folder_id,uidvalidity,uid,moving FROM messages
             WHERE (account_id,thread_id)=(?1,?2) ORDER BY folder_id,uid",
        )?
        .query_map(params![ALICE, fil], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(lettres)
}

fn arrivee(lot: &Lot) -> Option<&str> {
    match &lot.action {
        Action::Deplacement(arrivee) => arrivee.as_ref().map(|a| a.chemin.as_str()),
        Action::Drapeau { .. } => panic!("Lot de déplacement attendu."),
    }
}

fn deplacees(generation: u32, sorts: &[(u32, Sort)]) -> Depot {
    Depot::Deplace {
        generation: Some(generation),
        sorts: sorts.iter().copied().collect(),
    }
}

#[test]
fn archiver_retire_les_fils_de_leur_liste_des_l_enregistrement() -> Result<(), Error> {
    let app = classeur()?;
    let avant = revision(&app, ALICE)?;
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c", "1/fil"]);
    let operation = enregistrer(&app, ALICE, &archivage(A, &["1/fil", "1/c"]))?;
    assert_eq!(operation["type"], "archiver");
    assert_eq!(operation["etat"], "en_attente");
    assert_eq!(
        resultats(&operation),
        [("1/fil", "en_attente"), ("1/c", "en_attente")]
    );
    // Les fils B et C ont quitté Réception. Le fil A y garde son état de lecture.
    assert_eq!(fils(&app, RECEPTION)?, ["1/a"]);
    let (total, non_lus): (i64, i64) = app.db.reader()?.query_row(
        "SELECT c.total,sum(t.unread) FROM list_counts c JOIN list_threads t USING(account_id,scope)
         WHERE (c.account_id,c.scope)=(?1,?2)",
        params![ALICE, RECEPTION],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    assert_eq!((total, non_lus), (1, 0));
    // La lettre du fil B rangée dans Atelier n’est pas concernée.
    assert_eq!(fils(&app, "dossier/2")?, ["1/fil"]);
    assert_eq!(
        lettres(&app, "1/fil")?,
        [
            (INBOX, 1, 3, true),
            (INBOX, 1, 4, true),
            (ATELIER, 1, 7, false)
        ]
    );
    assert!(fils(&app, CLASSEES)?.is_empty());
    let apres = revision(&app, ALICE)?;
    assert_ne!(apres, avant);
    assert_eq!(operation["revision"], apres);

    // Le rejeu rend l’opération connue sans rien retirer d’autre.
    let rejeu = enregistrer(&app, ALICE, &archivage(A, &["1/a"]))?;
    assert_eq!(rejeu, operation);
    assert_eq!(revision(&app, ALICE)?, apres);
    assert_eq!(fils(&app, RECEPTION)?, ["1/a"]);

    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.chemin.as_deref(), Some("INBOX"));
    assert_eq!(lot.uids, [3, 4, 5]);
    assert_eq!(arrivee(&lot), Some("Archives"));
    Ok(())
}

#[test]
fn la_lettre_confirmee_rejoint_les_archives_et_la_lettre_refusee_revient() -> Result<(), Error> {
    let app = classeur()?;
    // Le rendu gardé pour la lettre 3 porte sa clé de départ.
    app.db.writer()?.execute_batch(
        "INSERT INTO message_bodies VALUES (1,1,1,3,1,'{}');
         INSERT INTO message_images VALUES (1,1,1,3,2,'image/png',x'00');",
    )?;
    let operation = enregistrer(&app, ALICE, &archivage(A, &["1/fil", "1/c"]))?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    let depot = deplacees(
        GENERATION,
        &[
            (3, Sort::Partie(Some(11))),
            (4, Sort::Partie(Some(12))),
            (5, Sort::Restee),
        ],
    );
    conclure(&app.db, ALICE, &lot, &depot)?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(
        resultats(&close),
        [("1/fil", "confirme"), ("1/c", "refuse")]
    );
    assert_ne!(close["revision"], operation["revision"]);
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c"]);
    assert_eq!(fils(&app, CLASSEES)?, ["1/fil"]);
    assert_eq!(
        lettres(&app, "1/fil")?,
        [
            (ATELIER, 1, 7, false),
            (ARCHIVES, GENERATION, 11, false),
            (ARCHIVES, GENERATION, 12, false)
        ]
    );
    assert_eq!(lettres(&app, "1/c")?, [(INBOX, 1, 5, false)]);
    // Chaque lettre garde ses drapeaux. Le rendu de l’ancienne clé est retiré.
    assert_eq!(drapeaux(&app, ARCHIVES, 11)?, "[]");
    assert_eq!(drapeaux(&app, ARCHIVES, 12)?, r#"["\\Seen"]"#);
    let rendus: i64 = app.db.reader()?.query_row(
        "SELECT (SELECT count(*) FROM message_bodies)+(SELECT count(*) FROM message_images)",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(rendus, 0);
    assert!(prochain(&app.db, ALICE)?.is_none());
    Ok(())
}

#[test]
fn sans_nouvel_uid_fiable_la_lettre_quitte_la_base_jusqu_au_prochain_import() -> Result<(), Error> {
    let app = classeur()?;
    // La lettre 12 est déjà connue dans les archives.
    app.db.writer()?.execute_batch(
        "INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,subject,
             received_at,recipients,flags,size,has_attachments,snippet)
         VALUES (1,4,7,12,'1/fil','Déjà importée',101,'{}','[]',5,0,'');",
    )?;
    enregistrer(&app, ALICE, &archivage(A, &["1/fil", "1/c"]))?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    let depot = deplacees(
        GENERATION,
        &[
            (3, Sort::Partie(None)),
            (4, Sort::Partie(Some(12))),
            (5, Sort::Partie(Some(13))),
        ],
    );
    conclure(&app.db, ALICE, &lot, &depot)?;
    assert_eq!(
        resultats(&connue(&app, ALICE, A)?),
        [("1/fil", "confirme"), ("1/c", "confirme")]
    );
    assert_eq!(
        lettres(&app, "1/fil")?,
        [(ATELIER, 1, 7, false), (ARCHIVES, GENERATION, 12, false)]
    );
    assert_eq!(drapeaux(&app, ARCHIVES, 12)?, "[]");
    assert_eq!(lettres(&app, "1/c")?, [(ARCHIVES, GENERATION, 13, false)]);

    // Les UID annoncés pour une autre génération ne désignent pas ces lettres.
    enregistrer(&app, ALICE, &archivage(B, &["1/a"]))?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    let autre_generation = deplacees(8, &[(6, Sort::Partie(Some(14)))]);
    conclure(&app.db, ALICE, &lot, &autre_generation)?;
    assert_eq!(connue(&app, ALICE, B)?["etat"], "confirmee");
    assert!(lettres(&app, "1/a")?.is_empty());
    assert!(fils(&app, RECEPTION)?.is_empty());
    Ok(())
}

#[test]
fn une_lettre_disparue_compte_comme_deplacee_si_la_commande_etait_partie() -> Result<(), Error> {
    let app = classeur()?;
    let cibles = ["1-1-3", "1-1-5"];
    enregistrer(&app, ALICE, &demande(A, Genre::Archiver, None, &cibles))?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.uids, [3, 5]);
    // La commande est partie pour la lettre 5, puis la connexion a été coupée.
    tenter(&app.db, &lot, &[5])?;
    let aucune = Depot::Deplace {
        generation: None,
        sorts: BTreeMap::new(),
    };
    conclure(&app.db, ALICE, &lot, &aucune)?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(
        resultats(&close),
        [("1-1-3", "absent"), ("1-1-5", "confirme")]
    );
    // Aucune des deux n’est plus dans son dossier chez l’hébergeur.
    assert_eq!(
        lettres(&app, "1/fil")?,
        [(INBOX, 1, 4, false), (ATELIER, 1, 7, false)]
    );
    assert!(lettres(&app, "1/c")?.is_empty());
    Ok(())
}

#[test]
fn un_refus_rend_les_lettres_a_leur_liste() -> Result<(), Error> {
    let app = classeur()?;
    let operation = enregistrer(&app, ALICE, &archivage(A, &["1/fil", "1/c"]))?;
    assert_eq!(deposer(&app, ALICE, Depot::Refuse)?, [3, 4, 5]);
    let refusee = connue(&app, ALICE, A)?;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(
        resultats(&refusee),
        [("1/fil", "refuse"), ("1/c", "refuse")]
    );
    assert_ne!(refusee["revision"], operation["revision"]);
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c", "1/fil"]);
    assert_eq!(
        lettres(&app, "1/fil")?,
        [
            (INBOX, 1, 3, false),
            (INBOX, 1, 4, false),
            (ATELIER, 1, 7, false)
        ]
    );
    assert!(fils(&app, CLASSEES)?.is_empty());
    Ok(())
}

#[test]
fn sans_dossier_d_archives_rien_n_est_enregistre() -> Result<(), Error> {
    let app = boite()?;
    let avant = revision(&app, ALICE)?;
    let sans_arrivee = enregistrer(&app, ALICE, &archivage(A, &["1/fil"]))?;
    assert_eq!(sans_arrivee, Value::Null);
    assert_eq!(connue(&app, ALICE, A)?, Value::Null);
    assert_eq!(revision(&app, ALICE)?, avant);
    assert_eq!(fils(&app, RECEPTION)?, ["1/fil"]);

    // Le dossier choisi par l’utilisateur reçoit les archives.
    app.db.writer()?.execute(
        "INSERT INTO archive_choices VALUES (?1,?2)",
        [ALICE, ATELIER],
    )?;
    // Les lettres qui s’y trouvent déjà n’ont pas à être déplacées.
    let sur_place = demande(A, Genre::Archiver, Some("dossier/2"), &["1/fil"]);
    let refusee = enregistrer(&app, ALICE, &sur_place)?;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(resultats(&refusee), [("1/fil", "absent")]);
    let lettre = demande(B, Genre::Archiver, None, &["2-1-7", "1-1-3"]);
    let operation = enregistrer(&app, ALICE, &lettre)?;
    assert_eq!(
        resultats(&operation),
        [("2-1-7", "absent"), ("1-1-3", "en_attente")]
    );
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&lot), Some("Atelier"));
    assert_eq!(lot.uids, [3]);
    Ok(())
}

#[test]
fn le_choix_de_l_utilisateur_l_emporte_sur_le_role_decouvert() -> Result<(), Error> {
    let app = classeur()?;
    app.db.writer()?.execute(
        "INSERT INTO archive_choices VALUES (?1,?2)",
        [ALICE, ATELIER],
    )?;
    enregistrer(&app, ALICE, &archivage(A, &["1/c"]))?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&lot), Some("Atelier"));
    // Le dossier choisi disparaît avant l’envoi : le lot n’a plus d’arrivée.
    app.db
        .writer()?
        .execute("DELETE FROM folders WHERE id=?1", [ATELIER])?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&lot), None);
    assert_eq!(lot.uids, [5]);
    // Sans choix, le rôle découvert reprend la main pour l’opération suivante.
    enregistrer(&app, ALICE, &archivage(B, &["1/a"]))?;
    conclure(&app.db, ALICE, &lot, &Depot::Refuse)?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&lot), Some("Archives"));
    Ok(())
}

#[test]
fn un_dossier_de_depart_regenere_fait_oublier_ses_lettres() -> Result<(), Error> {
    let app = classeur()?;
    enregistrer(&app, ALICE, &demande(A, Genre::Archiver, None, &["2-1-7"]))?;
    app.db
        .writer()?
        .execute("UPDATE folders SET uidvalidity=2 WHERE id=?1", [ATELIER])?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.chemin, None);
    conclure(&app.db, ALICE, &lot, &Depot::Applique(BTreeMap::new()))?;
    let operation = connue(&app, ALICE, A)?;
    assert_eq!(operation["etat"], "refusee");
    assert_eq!(resultats(&operation), [("2-1-7", "absent")]);
    assert_eq!(
        lettres(&app, "1/fil")?,
        [(INBOX, 1, 3, false), (INBOX, 1, 4, false)]
    );
    Ok(())
}

#[test]
fn une_etoile_refusee_est_retiree_de_la_lettre_en_partance() -> Result<(), Error> {
    let app = classeur()?;
    enregistrer(&app, ALICE, &demande(A, Genre::Etoile, None, &["1-1-5"]))?;
    enregistrer(&app, ALICE, &archivage(B, &["1/c"]))?;
    assert_eq!(drapeaux(&app, INBOX, 5)?, r#"["\\Flagged"]"#);
    assert_eq!(lettres(&app, "1/c")?, [(INBOX, 1, 5, true)]);
    // Les opérations passent dans leur ordre : l’étoile d’abord, refusée.
    deposer(&app, ALICE, Depot::Refuse)?;
    assert_eq!(drapeaux(&app, INBOX, 5)?, "[]");
    assert_eq!(lettres(&app, "1/c")?, [(INBOX, 1, 5, true)]);
    let lot = prochain(&app.db, ALICE)?.unwrap();
    conclure(
        &app.db,
        ALICE,
        &lot,
        &deplacees(GENERATION, &[(5, Sort::Partie(Some(11)))]),
    )?;
    assert_eq!(drapeaux(&app, ARCHIVES, 11)?, "[]");
    // Une étoile confirmée suit la lettre dans son dossier d’arrivée.
    enregistrer(
        &app,
        ALICE,
        &demande(super::C, Genre::Etoile, None, &["1-1-6"]),
    )?;
    deposer(&app, ALICE, confirmes(&[6]))?;
    let derniere = "0190a1b2-0000-7000-8000-00000000000d";
    enregistrer(&app, ALICE, &archivage(derniere, &["1/a"]))?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    conclure(
        &app.db,
        ALICE,
        &lot,
        &deplacees(GENERATION, &[(6, Sort::Partie(Some(12)))]),
    )?;
    assert_eq!(drapeaux(&app, ARCHIVES, 12)?, r#"["\\Seen","\\Flagged"]"#);
    Ok(())
}
