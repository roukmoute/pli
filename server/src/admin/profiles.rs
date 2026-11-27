use std::net::IpAddr;

use rusqlite::{
    Connection, OptionalExtension, Row, named_params,
    types::{FromSql, FromSqlError, FromSqlResult, Type, ValueRef},
};
use serde::{Deserialize, Serialize};
use tokio_rustls::rustls::pki_types::ServerName;

use crate::Error;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TlsMode {
    Implicit,
    Starttls,
}

impl TlsMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Implicit => "implicit",
            Self::Starttls => "starttls",
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Endpoint {
    pub host: String,
    pub port: u16,
    pub tls: TlsMode,
}

impl Endpoint {
    fn new(host: &str, port: u16, tls: TlsMode) -> Self {
        Self {
            host: host.into(),
            port,
            tls,
        }
    }

    fn valid(&self, local: bool) -> bool {
        if local {
            self.port != 0 && self.host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
        } else {
            self.port != 0 && ServerName::try_from(self.host.as_str()).is_ok()
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub name: String,
    pub imap: Endpoint,
    pub smtp: Endpoint,
    pub domains: Vec<String>,
    pub active: bool,
    #[serde(default)]
    pub local_test: bool,
}

#[derive(PartialEq, Serialize)]
pub(crate) struct Server {
    pub id: i64,
    #[serde(flatten)]
    pub profile: Profile,
}

impl Profile {
    pub fn presets(allow_test: bool) -> Vec<Self> {
        let host = "mail.infomaniak.com";
        let mut profiles = vec![Self {
            name: "Infomaniak".into(),
            imap: Endpoint::new(host, 993, TlsMode::Implicit),
            smtp: Endpoint::new(host, 587, TlsMode::Starttls),
            domains: vec![],
            active: true,
            local_test: false,
        }];
        if allow_test {
            profiles.push(Self {
                name: "Banc local".into(),
                imap: Endpoint::new("127.0.0.1", 3993, TlsMode::Implicit),
                smtp: Endpoint::new("127.0.0.1", 3465, TlsMode::Implicit),
                domains: vec!["recette.test".into()],
                active: true,
                local_test: true,
            });
        }
        profiles
    }

    pub fn validate(&self, allow_test: bool) -> Result<(), &'static str> {
        if self.local_test && !allow_test {
            return Err("Le profil de test est désactivé.");
        }
        if self.name.trim().is_empty() {
            return Err("Donnez un nom au profil.");
        }
        if ![&self.imap, &self.smtp]
            .iter()
            .all(|ep| ep.valid(self.local_test))
        {
            return Err(
                "Vérifiez les hôtes et les ports. Le banc exige une adresse IP de bouclage.",
            );
        }
        if !self
            .domains
            .iter()
            .all(|d| matches!(ServerName::try_from(d.as_str()), Ok(ServerName::DnsName(_))))
        {
            return Err("Un domaine associé est invalide.");
        }
        Ok(())
    }
}

const COLUMNS: &str = "id, name, imap_host, imap_port, imap_tls, smtp_host, smtp_port, smtp_tls, domains, active, local_test";

impl FromSql for TlsMode {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "implicit" => Ok(Self::Implicit),
            "starttls" => Ok(Self::Starttls),
            _ => Err(FromSqlError::Other("Mode TLS inconnu".into())),
        }
    }
}

fn read_endpoint(row: &Row<'_>, [host, port, tls]: [&str; 3]) -> rusqlite::Result<Endpoint> {
    Ok(Endpoint {
        host: row.get(host)?,
        port: row.get(port)?,
        tls: row.get(tls)?,
    })
}

fn read_server(row: &Row<'_>) -> rusqlite::Result<Server> {
    let column = row.as_ref().column_index("domains")?;
    let domains = serde_json::from_str(row.get_ref(column)?.as_str()?)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, e.into()))?;
    Ok(Server {
        id: row.get("id")?,
        profile: Profile {
            name: row.get("name")?,
            imap: read_endpoint(row, ["imap_host", "imap_port", "imap_tls"])?,
            smtp: read_endpoint(row, ["smtp_host", "smtp_port", "smtp_tls"])?,
            domains,
            active: row.get("active")?,
            local_test: row.get("local_test")?,
        },
    })
}

pub(super) fn list(db: &Connection) -> rusqlite::Result<Vec<Server>> {
    db.prepare(&format!(
        "SELECT {COLUMNS} FROM server_profiles ORDER BY id"
    ))?
    .query_map([], read_server)?
    .collect()
}

pub(crate) fn for_domain(db: &Connection, domain: &str) -> Result<Option<Server>, Error> {
    let mut statement = db.prepare(&format!(
        "SELECT {COLUMNS} FROM server_profiles
         WHERE active = 1 AND EXISTS (SELECT 1 FROM json_each(domains) WHERE value = ?1 COLLATE NOCASE)"
    ))?;
    let mut matches = statement.query_map([domain], read_server)?;
    let server = matches.next().transpose()?;
    if matches.next().is_some() {
        return Err("Plusieurs profils actifs correspondent au domaine.".into());
    }
    Ok(server)
}

pub(super) fn save(
    db: &Connection,
    id: Option<i64>,
    profile: Profile,
) -> Result<Option<Server>, Error> {
    let sql = match id {
        None => format!(
            "INSERT INTO server_profiles ({COLUMNS})
            VALUES (:id, :name, :imap_host, :imap_port, :imap_tls, :smtp_host, :smtp_port,
                    :smtp_tls, :domains, :active, :local_test) RETURNING id"
        ),
        Some(_) => "UPDATE server_profiles SET name=:name, imap_host=:imap_host,
            imap_port=:imap_port, imap_tls=:imap_tls, smtp_host=:smtp_host, smtp_port=:smtp_port,
            smtp_tls=:smtp_tls, domains=:domains, active=:active, local_test=:local_test
            WHERE id=:id RETURNING id"
            .into(),
    };
    let fields = named_params! {
        ":id": id, ":name": profile.name,
        ":imap_host": profile.imap.host, ":imap_port": profile.imap.port, ":imap_tls": profile.imap.tls.as_str(),
        ":smtp_host": profile.smtp.host, ":smtp_port": profile.smtp.port, ":smtp_tls": profile.smtp.tls.as_str(),
        ":domains": serde_json::to_string(&profile.domains)?, ":active": profile.active, ":local_test": profile.local_test,
    };
    let id = db.query_row(&sql, fields, |row| row.get(0)).optional()?;
    Ok(id.map(|id| Server { id, profile }))
}
