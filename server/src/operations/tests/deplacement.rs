use serde_json::Value;

use super::{
    A, ALICE, ATELIER, B, C, INBOX, INBOX_DE_BOB,
    archivage::{arrivee, classeur, deplacees, fils, lettres},
    connue, demande, deposer, drapeaux, enregistrer, resultats, revision,
};
use crate::{
    Error,
    operations::stockage::{Action, Demande, Depot, Genre, Sort, conclure, prochain},
};

const INCONNU: i64 = 99;
const RECEPTION: &str = "reception";
const RANGEES: &str = "dossier/2";

fn vers(id: &str, genre: Genre, dossier: Option<i64>, cibles: &[&str]) -> Demande {
    Demande {
        destination: dossier,
        ..demande(id, genre, Some(RECEPTION), cibles)
    }
}

#[test]
fn deplacer_ou_copier_exige_un_dossier_d_arrivee_ouvrable() -> Result<(), Error> {
    let app = classeur()?;
    app.db.writer()?.execute_batch(
        "INSERT INTO folders(id,account_id,remote_path) VALUES (7,1,'Classement');",
    )?;
    let avant = revision(&app, ALICE)?;
    // Aucun dossier, un dossier d’un autre compte, fermé ou inconnu : rien n’est enregistré.
    for genre in [Genre::Deplacer, Genre::Copier] {
        for dossier in [None, Some(INBOX_DE_BOB), Some(7), Some(INCONNU)] {
            let refusee = enregistrer(&app, ALICE, &vers(A, genre, dossier, &["1/c"]))?;
            assert_eq!(refusee, "Destination", "{dossier:?}");
        }
    }
    assert_eq!(connue(&app, ALICE, A)?, Value::Null);
    assert_eq!(revision(&app, ALICE)?, avant);
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c", "1/fil"]);
    Ok(())
}

#[test]
fn deplacer_range_les_fils_dans_le_dossier_choisi() -> Result<(), Error> {
    let app = classeur()?;
    let deplacement = vers(A, Genre::Deplacer, Some(ATELIER), &["1/c", "1/fil"]);
    let operation = enregistrer(&app, ALICE, &deplacement)?;
    assert_eq!(operation["type"], "deplacer");
    assert_eq!(operation["etat"], "en_attente");
    // Les fils quittent Réception dès l’enregistrement.
    assert_eq!(fils(&app, RECEPTION)?, ["1/a"]);
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert!(matches!(lot.action, Action::Deplacement(_)));
    assert_eq!(arrivee(&lot), Some("Atelier"));
    assert_eq!(lot.uids, [3, 4, 5]);
    let depot = deplacees(
        1,
        &[
            (3, Sort::Partie(Some(8))),
            (4, Sort::Restee),
            (5, Sort::Partie(Some(9))),
        ],
    );
    conclure(&app.db, ALICE, &lot, &depot)?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    // Chaque fil dit son issue : entièrement rangé, ou seulement en partie.
    assert_eq!(
        resultats(&close),
        [("1/c", "confirme"), ("1/fil", "partiel")]
    );
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/fil"]);
    assert_eq!(fils(&app, RANGEES)?, ["1/c", "1/fil"]);
    assert_eq!(lettres(&app, "1/c")?, [(ATELIER, 1, 9, false)]);

    // Une lettre déjà rangée dans le dossier choisi n’a pas à bouger.
    let sur_place = Demande {
        destination: Some(ATELIER),
        ..demande(B, Genre::Deplacer, None, &["2-1-7"])
    };
    let refusee = enregistrer(&app, ALICE, &sur_place)?;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(resultats(&refusee), [("2-1-7", "absent")]);
    Ok(())
}

#[test]
fn copier_laisse_la_lettre_en_place_et_l_ajoute_au_dossier_choisi() -> Result<(), Error> {
    let app = classeur()?;
    app.db.writer()?.execute(
        "UPDATE messages SET flags='[\"\\\\Flagged\"]' WHERE (folder_id,uid)=(?1,5)",
        [INBOX],
    )?;
    let avant = revision(&app, ALICE)?;
    let copie = vers(A, Genre::Copier, Some(ATELIER), &["1/c", "1/a"]);
    let operation = enregistrer(&app, ALICE, &copie)?;
    assert_eq!(operation["type"], "copier");
    assert_eq!(operation["etat"], "en_attente");
    // Rien ne quitte la liste : la copie n’y change rien avant sa confirmation.
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c", "1/fil"]);
    assert_eq!(lettres(&app, "1/c")?, [(INBOX, 1, 5, false)]);
    assert_eq!(revision(&app, ALICE)?, avant);
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert!(matches!(lot.action, Action::Copie(_)));
    assert_eq!(lot.chemin.as_deref(), Some("INBOX"));
    assert_eq!(arrivee(&lot), Some("Atelier"));
    assert_eq!(lot.uids, [5, 6]);
    // La lettre 5 reçoit son UID d’arrivée. Celui de la lettre 6 n’est pas annoncé.
    let depot = deplacees(1, &[(5, Sort::Partie(Some(9))), (6, Sort::Partie(None))]);
    conclure(&app.db, ALICE, &lot, &depot)?;
    let close = connue(&app, ALICE, A)?;
    assert_eq!(close["etat"], "confirmee");
    assert_eq!(
        resultats(&close),
        [("1/c", "confirme"), ("1/a", "confirme")]
    );
    // L’original reste à sa place et la copie garde ses drapeaux.
    assert_eq!(
        lettres(&app, "1/c")?,
        [(INBOX, 1, 5, false), (ATELIER, 1, 9, false)]
    );
    assert_eq!(drapeaux(&app, ATELIER, 9)?, r#"["\\Flagged"]"#);
    assert_eq!(lettres(&app, "1/a")?, [(INBOX, 1, 6, false)]);
    assert_eq!(fils(&app, RECEPTION)?, ["1/a", "1/c", "1/fil"]);
    assert_eq!(fils(&app, RANGEES)?, ["1/c", "1/fil"]);
    assert_ne!(revision(&app, ALICE)?, avant);

    // Refusée par la messagerie, la copie n’ajoute rien.
    enregistrer(
        &app,
        ALICE,
        &vers(B, Genre::Copier, Some(ATELIER), &["1/a"]),
    )?;
    assert_eq!(deposer(&app, ALICE, Depot::Refuse)?, [6]);
    let refusee = connue(&app, ALICE, B)?;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(resultats(&refusee), [("1/a", "refuse")]);
    assert_eq!(lettres(&app, "1/a")?, [(INBOX, 1, 6, false)]);

    // Le dossier choisi disparaît avant l’envoi : le lot n’a plus d’arrivée.
    enregistrer(
        &app,
        ALICE,
        &vers(C, Genre::Copier, Some(ATELIER), &["1/a"]),
    )?;
    app.db
        .writer()?
        .execute("DELETE FROM folders WHERE id=?1", [ATELIER])?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert!(matches!(lot.action, Action::Copie(None)));
    Ok(())
}

#[test]
fn une_copie_vers_la_boite_de_reception_s_y_lit_sans_passer_par_le_tri() -> Result<(), Error> {
    let app = classeur()?;
    let copie = Demande {
        destination: Some(INBOX),
        ..demande(A, Genre::Copier, Some(RANGEES), &["1/fil"])
    };
    enregistrer(&app, ALICE, &copie)?;
    let lot = prochain(&app.db, ALICE)?.unwrap();
    assert_eq!(lot.chemin.as_deref(), Some("Atelier"));
    assert_eq!(arrivee(&lot), Some("INBOX"));
    assert_eq!(lot.uids, [7]);
    // L’UID d’arrivée dépasse le repère de la boîte.
    conclure(
        &app.db,
        ALICE,
        &lot,
        &deplacees(1, &[(7, Sort::Partie(Some(40)))]),
    )?;
    assert_eq!(
        lettres(&app, "1/fil")?,
        [
            (INBOX, 1, 3, false),
            (INBOX, 1, 4, false),
            (INBOX, 1, 40, false),
            (ATELIER, 1, 7, false)
        ]
    );
    let lettres_en_reception: i64 = app.db.reader()?.query_row(
        "SELECT message_count FROM list_threads WHERE (account_id,item,scope)=(?1,'1/fil',?2)",
        rusqlite::params![ALICE, RECEPTION],
        |r| r.get(0),
    )?;
    assert_eq!(lettres_en_reception, 3);
    Ok(())
}
