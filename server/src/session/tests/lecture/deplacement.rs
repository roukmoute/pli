use tokio::net::TcpListener;

use super::{
    archivage::{LETTRES, RECEPTION, depart, ids, presentes},
    operations::{Banc, OPERATIONS, banc, messagerie, sans_autre_commande},
    *,
};

const A: &str = "0190a1b2-0000-7000-8000-0000000000c1";
const B: &str = "0190a1b2-0000-7000-8000-0000000000c2";
const C: &str = "0190a1b2-0000-7000-8000-0000000000c3";
const RANGEES: &str = "/api/v1/folders/2/messages";
const ATELIER: i64 = 2;
const INCONNU: i64 = 99;

// Réception d’Alice : les fils A, C et B. Atelier est son dossier personnel.
async fn boite() -> Result<(Banc, TcpListener), Error> {
    let (banc, listener) = banc().await?;
    banc.app.db.writer()?.execute_batch(LETTRES)?;
    Ok((banc, listener))
}

fn vers(id: &str, genre: &str, dossier: Value, fils: &[&str]) -> Value {
    json!({"id": id, "type": genre, "vue": "reception", "cibles": fils, "destination": dossier})
}

// Une copie n’efface rien : elle n’a pas besoin de connaître les capacités du serveur.
fn copie(avant: &str, reponse: &str) -> Dialogue {
    let mut dialogue = selection("SELECT", 1);
    dialogue.push(presentes(avant, &format!(" {}", avant.replace(',', " "))));
    dialogue.push((
        format!("UID COPY {avant} \"Atelier\""),
        format!("{{tag}} {reponse}\r\n"),
    ));
    dialogue
}

#[tokio::test]
async fn deplace_le_fil_vers_le_dossier_choisi() -> Result<(), Error> {
    let (banc, listener) = boite().await?;
    let deplacement = depart(
        "5",
        " MOVE",
        vec![
            ok("UID MOVE 5 \"Atelier\"", "* OK [COPYUID 1 5 8] rangée\r\n"),
            presentes("5", ""),
        ],
    );
    let (pair, _suivi) = messagerie(listener, vec![deplacement]);
    // Sans dossier d’arrivée, ou avec un dossier disparu, la demande dit ce qui manque.
    for dossier in [Value::Null, json!(INCONNU)] {
        let refusee = banc
            .envoyer(OPERATIONS, vers(A, "deplacer", dossier, &["1/c"]))
            .await;
        assert_eq!(refusee.status, StatusCode::CONFLICT);
        assert_eq!(refusee.body["code"], "destination_inconnue");
    }
    assert_eq!(banc.etat(A).await["code"], "operation_introuvable");

    let reponse = banc
        .envoyer(OPERATIONS, vers(A, "deplacer", json!(ATELIER), &["1/c"]))
        .await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(reponse.body["type"], "deplacer");
    assert_eq!(
        ids(&banc.lire(RECEPTION).await["nouveaux"]),
        ["1/a", "1/fil"]
    );
    assert_eq!(banc.conclue(A).await["etat"], "confirmee");
    assert_eq!(ids(&banc.lire(RANGEES).await["nouveaux"]), ["1/c"]);
    let fil = banc.lire("/api/v1/threads/1%2Fc").await;
    assert_eq!(fil[0]["id"], "2-1-8");
    sans_autre_commande(pair).await
}

#[tokio::test]
async fn copie_le_fil_sans_le_retirer_de_sa_liste() -> Result<(), Error> {
    let (banc, listener) = boite().await?;
    let (pair, _suivi) = messagerie(
        listener,
        vec![
            copie("5", "OK [COPYUID 1 5 9] copiée"),
            copie("6", "NO [OVERQUOTA] boîte pleine"),
        ],
    );
    let reponse = banc
        .envoyer(OPERATIONS, vers(B, "copier", json!(ATELIER), &["1/c"]))
        .await;
    assert_eq!(reponse.status, StatusCode::OK);
    assert_eq!(reponse.body["type"], "copier");
    let conclue = banc.conclue(B).await;
    assert_eq!(conclue["etat"], "confirmee");
    assert_eq!(
        conclue["resultats"],
        json!([{"id": "1/c", "etat": "confirme"}])
    );
    // L’original reste en Réception et la copie rejoint Atelier sous son propre UID.
    assert_eq!(
        ids(&banc.lire(RECEPTION).await["nouveaux"]),
        ["1/a", "1/c", "1/fil"]
    );
    assert_eq!(ids(&banc.lire(RANGEES).await["nouveaux"]), ["1/c"]);
    let fil = banc.lire("/api/v1/threads/1%2Fc").await;
    assert_eq!(
        json!([fil[0]["id"], fil[1]["id"]]),
        json!(["1-1-5", "2-1-9"])
    );

    // Refusée par la messagerie, la copie n’ajoute rien et le dit pour ce fil.
    banc.envoyer(OPERATIONS, vers(C, "copier", json!(ATELIER), &["1/a"]))
        .await;
    let refusee = banc.conclue(C).await;
    assert_eq!(refusee["etat"], "refusee");
    assert_eq!(
        refusee["resultats"],
        json!([{"id": "1/a", "etat": "refuse"}])
    );
    assert_eq!(ids(&banc.lire(RANGEES).await["nouveaux"]), ["1/c"]);
    sans_autre_commande(pair).await
}
