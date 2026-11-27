mod discovery;
mod header;
mod imap;
mod import;
mod lecture;
pub(crate) use lecture::Lecteur;
mod messages;
mod preview;
mod store;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{Json, extract::State};
use ring::aead::LessSafeKey;
use serde::Serialize;
use tokio::{
    sync::{mpsc, oneshot},
    task::{JoinHandle, spawn_blocking},
    time::timeout,
};

use crate::{
    Error,
    admin::{self, ImapSession, LoginFailure},
    db::Database,
    http::{ApiError, unavailable},
    session::{CompteAuthentifie, decrypt},
};

enum Request {
    Resume,
    Login(Box<ImapSession>, oneshot::Sender<bool>),
}

struct Worker {
    requests: mpsc::Sender<Request>,
    task: JoinHandle<()>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Context {
    db: Arc<Database>,
    key: Option<LessSafeKey>,
    allow_test: bool,
}

impl Context {
    async fn reconnect(self: &Arc<Self>, account: i64) -> Result<ImapSession, LoginFailure> {
        let context = self.clone();
        let (server, address, password) = spawn_blocking(move || -> Result<_, Error> {
            let db = context.db.reader()?;
            let (profile, address, mut encrypted): (i64, String, Vec<u8>) = db.query_row(
                "SELECT server_profile_id, address, encrypted_secret FROM accounts WHERE id=?1",
                [account],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let (_, domain) = address
                .rsplit_once('@')
                .ok_or("Domaine de compte absent.")?;
            let server = admin::for_domain(&db, domain)?
                .filter(|s| s.id == profile)
                .ok_or("Profil du compte inactif.")?;
            server.profile.validate(context.allow_test)?;
            let key = context.key.as_ref().ok_or("Clé de messagerie absente.")?;
            let password = decrypt(key, profile, &address, &mut encrypted)?;
            Ok((server, address, password))
        })
        .await
        .map_err(|_| LoginFailure::Unavailable)?
        .map_err(|_| LoginFailure::Unavailable)?;
        admin::login_mailbox(&server.profile, &address, &password).await
    }

    async fn discover(
        &self,
        account: i64,
        session: &mut ImapSession,
    ) -> Result<Vec<messages::Mailbox>, Error> {
        let discovery = discovery::discover(session).await?;
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut writer = db.writer()?;
            store::save(&mut writer, account, &discovery)?;
            Ok(messages::mailboxes(&writer, account)?)
        })
        .await?
    }

    async fn prepare(
        self: &Arc<Self>,
        account: i64,
        request: Request,
    ) -> Result<(ImapSession, Vec<messages::Mailbox>), LoginFailure> {
        let (mut session, reply) = match request {
            Request::Resume => (self.reconnect(account).await?, None),
            Request::Login(session, reply) => (*session, Some(reply)),
        };
        let folders = self.discover(account, &mut session).await;
        if let Some(reply) = reply {
            let _ = reply.send(folders.is_ok());
        }
        Ok((session, folders.map_err(|_| LoginFailure::Unavailable)?))
    }

    async fn run(self: Arc<Self>, account: i64, mut requests: mpsc::Receiver<Request>) {
        let mut next = requests.recv().await;
        while let Some(request) = next {
            let result = match self.prepare(account, request).await {
                Ok((session, folders)) => tokio::select! {
                    result = import::run(&self, session, &folders) => result.map_err(|_| LoginFailure::Unavailable),
                    request = requests.recv() => {
                        next = request;
                        continue;
                    }
                },
                Err(failure) => Err(failure),
            };
            next = if matches!(result, Err(LoginFailure::Unavailable)) {
                tracing::warn!(
                    account_id = account,
                    "Synchronisation du compte à reprendre"
                );
                timeout(Duration::from_secs(30), requests.recv())
                    .await
                    .unwrap_or(Some(Request::Resume))
            } else {
                // Après un refus du secret enregistré, attendre une nouvelle authentification.
                requests.recv().await
            };
        }
    }
}

pub(crate) struct Synchronizer {
    context: Arc<Context>,
    workers: Mutex<HashMap<i64, Worker>>,
}

impl Synchronizer {
    pub(crate) fn start(
        db: Arc<Database>,
        key: Option<LessSafeKey>,
        allow_test: bool,
    ) -> Result<Self, Error> {
        let sync = Self {
            context: Arc::new(Context {
                db,
                key,
                allow_test,
            }),
            workers: Mutex::default(),
        };
        if sync.context.key.is_some() {
            let reader = sync.context.db.reader()?;
            let mut query = reader.prepare("SELECT id FROM accounts")?;
            for id in query.query_map([], |r| r.get(0))? {
                sync.worker(id?)?
                    .try_send(Request::Resume)
                    .map_err(|_| "Synchroniseur indisponible.")?;
            }
        }
        Ok(sync)
    }

    fn worker(&self, account: i64) -> Result<mpsc::Sender<Request>, Error> {
        let mut workers = self
            .workers
            .lock()
            .map_err(|_| "Registre des tâches indisponible.")?;
        let worker = workers.entry(account).or_insert_with(|| {
            let (requests, receiver) = mpsc::channel(1);
            let task = tokio::spawn(self.context.clone().run(account, receiver));
            Worker { requests, task }
        });
        Ok(worker.requests.clone())
    }

    pub(crate) async fn on_login(
        &self,
        account: i64,
        session: ImapSession,
        has_marker: bool,
    ) -> Result<(), Error> {
        let (reply, result) = oneshot::channel();
        self.worker(account)?
            .send(Request::Login(Box::new(session), reply))
            .await
            .map_err(|_| "Synchroniseur arrêté.")?;
        if !has_marker && !result.await? {
            return Err("Premier rattachement incomplet.".into());
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub(crate) struct Folders {
    folders: Vec<store::FolderView>,
}

pub(crate) async fn folders(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
) -> Result<Json<Folders>, ApiError> {
    let folders = read(db, account, store::folders).await?;
    Ok(Json(Folders { folders }))
}

#[derive(Serialize)]
pub(crate) struct Screener {
    senders: Vec<messages::PendingSender>,
}

pub(crate) async fn screener(
    account: CompteAuthentifie,
    State(db): State<Arc<Database>>,
) -> Result<Json<Screener>, ApiError> {
    let senders = read(db, account, messages::pending).await?;
    Ok(Json(Screener { senders }))
}

async fn read<T: Send + 'static>(
    db: Arc<Database>,
    account: CompteAuthentifie,
    query: fn(&rusqlite::Connection, i64) -> rusqlite::Result<T>,
) -> Result<T, ApiError> {
    spawn_blocking(move || -> Result<_, Error> { Ok(query(&*db.reader()?, account.account_id)?) })
        .await
        .map_err(unavailable)?
        .map_err(unavailable)
}
