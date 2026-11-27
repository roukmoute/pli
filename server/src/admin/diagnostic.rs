use std::{future::Future, io, net::SocketAddr, time::Duration};

use async_imap::Client as ImapClient;
use mail_send::{
    Credentials as SmtpCredentials, SmtpClient,
    smtp::{AssertReply, tls::build_tls_connector},
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpStream, lookup_host},
    time::timeout,
};
use tokio_rustls::{client::TlsStream, rustls::pki_types::ServerName};

use super::profiles::{Endpoint, Profile, TlsMode};
use crate::Error;

const STEP_TIMEOUT: Duration = Duration::from_secs(5);

// Only the request owns these values. They cannot be serialized into a response or profile.
#[derive(Deserialize)]
pub(super) struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Default, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    #[default]
    Skipped,
    Ok,
    Failed,
}

impl Status {
    async fn check<T, E>(&mut self, operation: impl Future<Output = Result<T, E>>) -> Option<T> {
        match timeout(STEP_TIMEOUT, operation).await {
            Ok(Ok(value)) => {
                *self = Self::Ok;
                Some(value)
            }
            _ => {
                *self = Self::Failed;
                None
            }
        }
    }
}

#[derive(Default, Serialize)]
struct ProtocolReport {
    resolution: Status,
    connection: Status,
    tls: Status,
    authentication: Status,
}

#[derive(Serialize)]
pub(super) struct Diagnostic {
    imap: ProtocolReport,
    smtp: ProtocolReport,
}

impl Diagnostic {
    pub fn passed(&self) -> bool {
        self.imap.authentication == Status::Ok && self.smtp.authentication == Status::Ok
    }
}

pub(super) async fn diagnose(profile: &Profile, credentials: &Credentials) -> Diagnostic {
    let mut imap = ProtocolReport::default();
    let mut smtp = ProtocolReport::default();
    tokio::join!(
        imap.run(
            Protocol::Imap,
            &profile.imap,
            profile.local_test,
            credentials
        ),
        smtp.run(
            Protocol::Smtp,
            &profile.smtp,
            profile.local_test,
            credentials
        ),
    );
    Diagnostic { imap, smtp }
}

#[derive(Clone, Copy)]
enum Protocol {
    Imap,
    Smtp,
}

impl ProtocolReport {
    async fn run(
        &mut self,
        protocol: Protocol,
        ep: &Endpoint,
        local: bool,
        credentials: &Credentials,
    ) -> Option<()> {
        let addresses = self.resolution.check(resolve(ep)).await?;
        let stream = self
            .connection
            .check(TcpStream::connect(addresses.as_slice()))
            .await?;
        let secure = self.tls.check(protocol.secure(stream, ep, local)).await?;
        self.authentication
            .check(protocol.authenticate(secure, ep.tls, credentials))
            .await
    }
}

async fn resolve(ep: &Endpoint) -> io::Result<Vec<SocketAddr>> {
    let addresses: Vec<_> = lookup_host((ep.host.as_str(), ep.port)).await?.collect();
    if addresses.is_empty() {
        return Err(io::ErrorKind::NotFound.into());
    }
    Ok(addresses)
}

async fn imap_greeting<T>(client: &mut ImapClient<T>) -> async_imap::error::Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    client
        .read_response()
        .await?
        .ok_or(async_imap::error::Error::ConnectionLost)?;
    Ok(())
}

pub(crate) enum LoginFailure {
    InvalidCredentials,
    Unavailable,
}

async fn imap_login(
    stream: TlsStream<TcpStream>,
    mode: TlsMode,
    username: &str,
    password: &str,
) -> async_imap::error::Result<()> {
    let mut client = ImapClient::new(stream);
    if mode == TlsMode::Implicit {
        imap_greeting(&mut client).await?;
    }
    client.login(username, password).await.map_err(|(e, _)| e)?;
    Ok(())
}

fn login_failure(error: async_imap::error::Error) -> LoginFailure {
    use async_imap::error::Error as ImapError;
    match error {
        ImapError::No(detail) => {
            // async-imap 0.12 enveloppe les codes RFC 5530 non typés dans le texte de NO.
            let code = detail
                .split_once("info: Some(\"[")
                .and_then(|(_, info)| info.split_once(']'))
                .map(|(code, _)| code);
            match code.map(str::to_ascii_uppercase).as_deref() {
                Some("UNAVAILABLE" | "SERVERBUG" | "CONTACTADMIN") => LoginFailure::Unavailable,
                _ => LoginFailure::InvalidCredentials,
            }
        }
        ImapError::Validate(_) => LoginFailure::InvalidCredentials,
        _ => LoginFailure::Unavailable,
    }
}

pub(crate) async fn login_mailbox(
    profile: &Profile,
    address: &str,
    password: &str,
) -> Result<(), LoginFailure> {
    let ep = &profile.imap;
    timeout(STEP_TIMEOUT, async {
        let tcp = TcpStream::connect((ep.host.as_str(), ep.port))
            .await
            .map_err(|_| LoginFailure::Unavailable)?;
        let tls = Protocol::Imap
            .secure(tcp, ep, profile.local_test)
            .await
            .map_err(|_| LoginFailure::Unavailable)?;
        imap_login(tls, ep.tls, address, password)
            .await
            .map_err(login_failure)
    })
    .await
    .map_err(|_| LoginFailure::Unavailable)?
}

fn smtp_client<T: AsyncRead + AsyncWrite>(stream: T) -> SmtpClient<T> {
    SmtpClient {
        stream,
        timeout: STEP_TIMEOUT,
    }
}

impl Protocol {
    async fn starttls(self, stream: TcpStream) -> Result<TcpStream, Error> {
        match self {
            Self::Imap => {
                let mut client = ImapClient::new(stream);
                imap_greeting(&mut client).await?;
                client.run_command_and_check_ok("STARTTLS", None).await?;
                Ok(client.into_inner())
            }
            Self::Smtp => {
                let mut client = smtp_client(stream);
                client.read().await?.assert_code(220)?;
                if !client
                    .ehlo("localhost")
                    .await?
                    .has_capability(smtp_proto::EXT_START_TLS)
                {
                    return Err("STARTTLS indisponible".into());
                }
                client.cmd(b"STARTTLS\r\n").await?.assert_code(220)?;
                Ok(client.stream)
            }
        }
    }

    async fn secure(
        self,
        stream: TcpStream,
        ep: &Endpoint,
        local: bool,
    ) -> Result<TlsStream<TcpStream>, Error> {
        let stream = match ep.tls {
            TlsMode::Implicit => stream,
            TlsMode::Starttls => self.starttls(stream).await?,
        };
        let name = ServerName::try_from(ep.host.clone())?;
        // Local profiles have already been restricted to literal loopback addresses.
        Ok(build_tls_connector(local)?.connect(name, stream).await?)
    }

    async fn authenticate(
        self,
        stream: TlsStream<TcpStream>,
        mode: TlsMode,
        credentials: &Credentials,
    ) -> Result<(), Error> {
        match self {
            Self::Imap => {
                imap_login(stream, mode, &credentials.username, &credentials.password).await?;
            }
            Self::Smtp => {
                let mut client = smtp_client(stream);
                if mode == TlsMode::Implicit {
                    client.read().await?.assert_code(220)?;
                }
                let capabilities = client.ehlo("localhost").await?;
                let auth = SmtpCredentials::Plain {
                    username: &credentials.username,
                    secret: &credentials.password,
                };
                client.authenticate(&auth, &capabilities).await?;
            }
        }
        Ok(())
    }
}
