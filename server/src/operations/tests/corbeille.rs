use std::collections::BTreeMap;

use rusqlite::params;
use serde_json::Value;

use super::{
    A, ALICE, ATELIER, B, C, INBOX, INBOX_DE_BOB,
    archivage::{arrivee, classeur, deplacees, fils, lettres},
    connue, demande, enregistrer, resultats, revision,
};
use crate::{
    Error,
    operations::stockage::{Action, Demande, Depot, Genre, Sort, conclure, prochain},
    test_support::TestApp,
};

const CORBEILLE: i64 = 5;
const INCONNU: i64 = 99;
const RECEPTION: &str = "reception";
const JETEES: &str = "dossier/5";
const RECLAMES: &str = "dossier/6";
const DOSSIERS: &str =
    "INSERT INTO folders(id,account_id,remote_path,role,uidvalidity,uidnext) VALUES
    (5,1,'Papierkorb','trash',9,1),(6,1,'Spam','junk',3,1);";

// La Corbeille d’Alice : le fil X, jeté depuis Réception et depuis Atelier, et le fil Y
// dont l’origine est inconnue. Les Indésirables gardent une réclame venue de Réception.
fn corbeille_pleine() -> Result<TestApp, Error> {
    let app = classeur()?;
    let db = app.db.writer()?;
    db.execute_batch(DOSSIERS)?;
    db.execute_batch(
        "INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,sender_address,subject,
             received_at,recipients,flags,size,has_attachments,snippet,origin_folder) VALUES
             (1,5,9,21,'1/x','lea@recette.test','Jetée de Réception',200,'{}','[]',5,0,'',1),
             (1,5,9,22,'1/x',NULL,'Jetée d’Atelier',201,'{}','[]',5,0,'',2),
             (1,5,9,23,'1/y',NULL,'Sans origine',202,'{}','[]',5,0,'',NULL),
             (1,6,3,31,'1/z',NULL,'Réclame',203,'{}','[]',5,0,'',1);",
    )?;
    drop(db);
    Ok(app)
}

// UID et dossier d’origine retenu pour chaque lettre du dossier.
fn origines(app: &TestApp, dossier: i64) -> Result<Vec<(u32, Option<i64>)>, Error> {
    let db = app.db.reader()?;
    let origines = db
        .prepare("SELECT uid,origin_folder FROM messages WHERE folder_id=?1 ORDER BY uid")?
        .query_map([dossier], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(origines)
}

fn total(app: &TestApp, portee: &str) -> Result<i64, Error> {
    Ok(app.db.reader()?.query_row(
        "SELECT total FROM list_counts WHERE (account_id,scope)=(?1,?2)",
        params![ALICE, portee],
        |r| r.get(0),
    )?)
}

fn parties(sorts: &[(u32, Sort)]) -> Depot {
    Depot::Deplace {
        generation: None,
        sorts: sorts.iter().copied().collect(),
    }
}

#[test]
fn supprimer_range_dans_la_corbeille_et_retient_le_dossier_d_origine() -> Result<(), Error> {
    let app = classeur()?;
    let jeter = |id, cibles: &[&str]| demande(id, Genre::Corbeille, Some(RECEPTION), cibles);
    // Sans corbeille annoncée par la messagerie, rien n’est enregistré.
    assert_eq!(enregistrer(&app, ALICE, &jeter(A, &["1/c"]))?, "Corbeille");
    assert_eq!(connue(&app, ALICE, A)?, Value::Null);
    app.db.writer()?.execute_batch(DOSSIERS)?;

    let operation = enregistrer(&app, ALICE, &jeter(A, &["1/c", "1/fil"]))?;
    assert_eq!(operation["type"], "corbeille");
    assert_eq!(operation["etat"], "en_attente");
    assert_eq!(fils(&app, RECEPTION)?, ["1/a"]);
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&lot), Some("Papierkorb"));
    assert_eq!(lot.uids, [3, 4, 5]);
    let depot = deplacees(
        9,
        &[
            (3, Sort::Partie(Some(41))),
            (4, Sort::Partie(Some(42))),
            (5, Sort::Restee),
        ],
    );
    conclure(&app.db, ALICE, &lot, &depot)?;
    assert_eq!(
        resultats(&connue(&app, ALICE, A)?),
        [("1/c", "refuse"), ("1/fil", "confirme")]
    );
    assert_eq!(fils(&app, JETEES)?, ["1/fil"]);
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c"]);
    assert_eq!(
        origines(&app, CORBEILLE)?,
        [(41, Some(INBOX)), (42, Some(INBOX))]
    );

    // La lettre d’Atelier retient son propre dossier. Déjà jetée, une lettre ne bouge plus.
    let lettres_visees = demande(B, Genre::Corbeille, None, &["2-1-7", "5-9-41"]);
    let operation = enregistrer(&app, ALICE, &lettres_visees)?;
    assert_eq!(
        resultats(&operation),
        [("2-1-7", "en_attente"), ("5-9-41", "absent")]
    );
    let lot = prochain(&app.db, ALICE)?.unwrap();
    conclure(
        &app.db,
        ALICE,
        &lot,
        &deplacees(9, &[(7, Sort::Partie(Some(43)))]),
    )?;
    assert_eq!(origines(&app, CORBEILLE)?[2], (43, Some(ATELIER)));
    Ok(())
}

#[test]
fn restaurer_ramene_chaque_lettre_dans_son_dossier_d_origine() -> Result<(), Error> {
    let app = corbeille_pleine()?;
    assert_eq!((total(&app, JETEES)?, total(&app, RECEPTION)?), (2, 3));
    let restauration = demande(A, Genre::Restaurer, Some(JETEES), &["1/x"]);
    let operation = enregistrer(&app, ALICE, &restauration)?;
    assert_eq!(operation["type"], "restaurer");
    assert_eq!(operation["etat"], "en_attente");
    assert_eq!(fils(&app, JETEES)?, ["1/y"]);

    // Chaque dossier d’origine reçoit son lot.
    let vers_reception = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(vers_reception.chemin.as_deref(), Some("Papierkorb"));
    assert_eq!(arrivee(&vers_reception), Some("INBOX"));
    assert_eq!(vers_reception.uids, [21]);
    conclure(
        &app.db,
        ALICE,
        &vers_reception,
        &deplacees(1, &[(21, Sort::Partie(Some(31)))]),
    )?;
    assert_eq!(connue(&app, ALICE, A)?["etat"], "en_attente");
    let vers_atelier = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&vers_atelier), Some("Atelier"));
    assert_eq!(vers_atelier.uids, [22]);
    conclure(
        &app.db,
        ALICE,
        &vers_atelier,
        &deplacees(1, &[(22, Sort::Partie(Some(8)))]),
    )?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(resultats(&close), [("1/x", "confirme")]);
    assert_eq!(
        lettres(&app, "1/x")?,
        [(INBOX, 1, 31, false), (ATELIER, 1, 8, false)]
    );
    // Les deux compteurs suivent : un fil de moins en Corbeille, un de plus en Réception.
    assert_eq!((total(&app, JETEES)?, total(&app, RECEPTION)?), (1, 4));
    // Revenue chez elle, la lettre n’a plus d’origine à retenir.
    assert_eq!(origines(&app, INBOX)?.last(), Some(&(31, None)));

    // Son UID dépasse le repère de la boîte. Replacée par l’utilisateur, la lettre se lit
    // en Réception, même si son expéditeur attend ensuite d’être trié.
    assert_eq!(fils(&app, RECEPTION)?[0], "1/x");
    let db = app.db.writer()?;
    db.execute(
        "INSERT INTO sender_decisions(account_id,address) VALUES (?1,'lea@recette.test')",
        [ALICE],
    )?;
    drop(db);
    assert_eq!(fils(&app, RECEPTION)?[0], "1/x");
    // Un refus explicite de l’expéditeur couvre tout son courrier de la boîte.
    app.db
        .writer()?
        .execute("UPDATE sender_decisions SET state='refuse'", [])?;
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c", "1/fil"]);
    Ok(())
}

#[test]
fn restaurer_sans_origine_connue_attend_le_choix_d_un_dossier() -> Result<(), Error> {
    let app = corbeille_pleine()?;
    let avant = revision(&app, ALICE)?;
    let restaurer = |id, choix| Demande {
        destination: choix,
        ..demande(id, Genre::Restaurer, Some(JETEES), &["1/x", "1/y"])
    };
    // Un dossier d’un autre compte ou inconnu ne remplace pas l’origine manquante.
    for choix in [None, Some(INBOX_DE_BOB), Some(INCONNU)] {
        let refusee = enregistrer(&app, ALICE, &restaurer(A, choix))?;
        assert_eq!(refusee, "Origine", "{choix:?}");
    }
    assert_eq!(connue(&app, ALICE, A)?, Value::Null);
    assert_eq!(revision(&app, ALICE)?, avant);
    assert_eq!(fils(&app, JETEES)?, ["1/y", "1/x"]);

    // Le dossier choisi ne sert qu’aux lettres sans origine.
    let operation = enregistrer(&app, ALICE, &restaurer(A, Some(ATELIER)))?;
    assert_eq!(
        resultats(&operation),
        [("1/x", "en_attente"), ("1/y", "en_attente")]
    );
    assert!(fils(&app, JETEES)?.is_empty());
    let vers_reception = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&vers_reception), Some("INBOX"));
    assert_eq!(vers_reception.uids, [21]);
    conclure(&app.db, ALICE, &vers_reception, &Depot::Refuse)?;
    let vers_atelier = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(arrivee(&vers_atelier), Some("Atelier"));
    assert_eq!(vers_atelier.uids, [22, 23]);
    conclure(
        &app.db,
        ALICE,
        &vers_atelier,
        &deplacees(
            1,
            &[(22, Sort::Partie(Some(8))), (23, Sort::Partie(Some(9)))],
        ),
    )?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(resultats(&close), [("1/x", "partiel"), ("1/y", "confirme")]);
    // La lettre refusée retrouve la Corbeille avec son origine.
    assert_eq!(fils(&app, JETEES)?, ["1/x"]);
    assert_eq!(origines(&app, CORBEILLE)?, [(21, Some(INBOX))]);
    Ok(())
}

#[test]
fn restaurer_part_de_la_corbeille_ou_des_indesirables_vers_un_dossier_qui_existe_encore()
-> Result<(), Error> {
    let app = corbeille_pleine()?;
    // Une réclame des Indésirables revient en Réception.
    let reclame = demande(A, Genre::Restaurer, Some(RECLAMES), &["1/z"]);
    assert_eq!(enregistrer(&app, ALICE, &reclame)?["etat"], "en_attente");
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.chemin.as_deref(), Some("Spam"));
    assert_eq!(arrivee(&lot), Some("INBOX"));
    conclure(&app.db, ALICE, &lot, &Depot::Refuse)?;
    // Hors de ces deux dossiers, il n’y a rien à restaurer.
    let ailleurs = Demande {
        destination: Some(ATELIER),
        ..demande(B, Genre::Restaurer, None, &["1-1-3"])
    };
    let refusee = enregistrer(&app, ALICE, &ailleurs)?;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(resultats(&refusee), [("1-1-3", "absent")]);
    // Le dossier d’origine a disparu : la lettre attend un autre choix.
    app.db
        .writer()?
        .execute("DELETE FROM folders WHERE id=?1", [ATELIER])?;
    assert_eq!(origines(&app, CORBEILLE)?[1], (22, None));
    let orpheline = demande(C, Genre::Restaurer, None, &["5-9-22"]);
    assert_eq!(enregistrer(&app, ALICE, &orpheline)?, "Origine");
    Ok(())
}

#[test]
fn la_suppression_definitive_n_efface_que_les_lettres_ciblees_de_la_corbeille() -> Result<(), Error>
{
    let app = corbeille_pleine()?;
    let effacer =
        |id, vue, cibles: &[&str]| demande(id, Genre::SupprimerDefinitivement, vue, cibles);
    let operation = enregistrer(&app, ALICE, &effacer(A, Some(JETEES), &["1/x", "1/a"]))?;
    assert_eq!(operation["type"], "supprimer_definitivement");
    assert_eq!(
        resultats(&operation),
        [("1/x", "en_attente"), ("1/a", "absent")]
    );
    assert_eq!(fils(&app, JETEES)?, ["1/y"]);
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert!(matches!(lot.action, Action::Suppression));
    assert_eq!(lot.chemin.as_deref(), Some("Papierkorb"));
    assert_eq!(lot.uids, [21, 22]);
    // La messagerie efface la lettre 21 et garde la 22.
    let depot = parties(&[(21, Sort::Partie(None)), (22, Sort::Restee)]);
    conclure(&app.db, ALICE, &lot, &depot)?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(resultats(&close), [("1/x", "partiel"), ("1/a", "absent")]);
    assert_eq!(lettres(&app, "1/x")?, [(CORBEILLE, 9, 22, false)]);
    // La lettre que personne n’a ciblée n’a pas bougé.
    assert_eq!(lettres(&app, "1/y")?, [(CORBEILLE, 9, 23, false)]);

    // Hors de la Corbeille, rien ne s’efface pour de bon.
    let ailleurs = enregistrer(&app, ALICE, &effacer(B, None, &["1-1-3", "6-3-31"]))?;
    assert_eq!(ailleurs["etat"], "refusee");
    assert_eq!(
        resultats(&ailleurs),
        [("1-1-3", "absent"), ("6-3-31", "absent")]
    );
    assert_eq!(fils(&app, RECLAMES)?, ["1/z"]);
    assert!(prochain(&app.db, ALICE)?.is_none());
    Ok(())
}

#[test]
fn vider_un_dossier_vise_toutes_ses_lettres_connues() -> Result<(), Error> {
    let app = corbeille_pleine()?;
    // Seuls la Corbeille et les Indésirables de ce compte se vident.
    let dossiers = [JETEES, "dossier/2", "dossier/3", "dossier/99"];
    let vidage = demande(A, Genre::ViderDossier, None, &dossiers);
    let operation = enregistrer(&app, ALICE, &vidage)?;
    assert_eq!(operation["type"], "vider_dossier");
    assert_eq!(
        resultats(&operation),
        [
            (JETEES, "en_attente"),
            ("dossier/2", "absent"),
            ("dossier/3", "absent"),
            ("dossier/99", "absent")
        ]
    );
    assert!(fils(&app, JETEES)?.is_empty());
    assert_eq!(fils(&app, RECLAMES)?, ["1/z"]);
    assert_eq!(fils(&app, "dossier/2")?, ["1/fil"]);
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert!(matches!(lot.action, Action::Suppression));
    assert_eq!(lot.uids, [21, 22, 23]);
    let toutes = [21, 22, 23].map(|uid| (uid, Sort::Partie(None)));
    conclure(&app.db, ALICE, &lot, &parties(&toutes))?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(resultats(&close)[0], (JETEES, "confirme"));
    assert!(origines(&app, CORBEILLE)?.is_empty());

    // Les Indésirables se vident de la même façon.
    enregistrer(
        &app,
        ALICE,
        &demande(B, Genre::ViderDossier, None, &[RECLAMES]),
    )?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.chemin.as_deref(), Some("Spam"));
    assert_eq!(lot.uids, [31]);
    // Le dossier a disparu chez l’hébergeur : ses lettres sont oubliées.
    conclure(&app.db, ALICE, &lot, &Depot::Applique(BTreeMap::new()))?;
    assert!(fils(&app, RECLAMES)?.is_empty());
    assert_eq!(connue(&app, ALICE, B)?["etat"], "refusee");
    Ok(())
}
