mod pagination;
mod projection;

use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, header::COOKIE},
};
use ring::digest::{SHA256, digest};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::{
    Error,
    test_support::{BOITES_DE_TEST, TestApp},
};

const RECEPTION: &str = "/api/v1/views/reception";
fn jeton(compte: u8) -> [u8; 32] {
    [compte; 32]
}

fn session(db: &Connection, compte: u8) -> rusqlite::Result<usize> {
    db.execute(
        "INSERT INTO sessions VALUES(?1,?2,unixepoch()+3600)",
        params![digest(&SHA256, &jeton(compte)).as_ref(), compte],
    )
}

fn boite() -> Result<TestApp, Error> {
    let app = TestApp::new()?;
    {
        let db = app.db.writer()?;
        db.execute_batch(BOITES_DE_TEST)?;
        session(&db, 1)?;
        session(&db, 2)?;
    }
    Ok(app)
}

fn lettre(
    db: &Connection,
    dossier: i64,
    uid: i64,
    fil: &str,
    adresse: &str,
    flags: Value,
) -> rusqlite::Result<usize> {
    db.execute(
        "INSERT INTO messages(account_id,folder_id,uidvalidity,uid,thread_id,sender_address,
         sender_name,subject,received_at,recipients,flags,size,has_attachments,snippet)
         SELECT account_id,id,1,?2,?3,?4,'Noa','Lettre '||?2,100,'{}',?5,15,0,'Extrait '||?2
         FROM folders WHERE id=?1 ON CONFLICT(account_id,folder_id,uidvalidity,uid) DO NOTHING",
        params![dossier, uid, fil, adresse, flags.to_string()],
    )
}

async fn reponse(
    router: &Router,
    path: &str,
    compte: Option<u8>,
) -> Result<(StatusCode, Value), Error> {
    let mut request = Request::get(path);
    if let Some(id) = compte {
        request = request.header(
            COOKIE,
            format!("__Host-pli_session={}", hex::encode(jeton(id))),
        );
    }
    let res = router.clone().oneshot(request.body(Body::empty())?).await?;
    let status = res.status();
    Ok((
        status,
        serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await?)?,
    ))
}

async fn lire(router: &Router, path: &str) -> Result<Value, Error> {
    let (status, body) = reponse(router, path, Some(1)).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}
