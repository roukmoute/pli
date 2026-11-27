use async_imap::imap_proto::{Address, AttributeValue, Envelope, MessageSection, SectionPath};
use chrono::DateTime;
use mail_parser::{HeaderForm, MessageParser};
use serde::Serialize;

use super::preview::Structure;
use crate::Error;

pub(super) const ITEMS: &str = "(UID ENVELOPE FLAGS INTERNALDATE RFC822.SIZE BODYSTRUCTURE BODY.PEEK[HEADER.FIELDS (REFERENCES)])";

#[derive(Serialize)]
pub(super) struct Contact {
    pub address: String,
    pub name: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Recipients {
    to: Vec<Contact>,
    cc: Vec<Contact>,
    bcc: Vec<Contact>,
}

pub(super) struct Header {
    pub uid: u32,
    pub message_id: Option<String>,
    pub references: Vec<String>,
    pub sender: Option<Contact>,
    pub recipients: Recipients,
    pub subject: String,
    pub sent_at: Option<i64>,
    pub received_at: i64,
    pub flags: Vec<String>,
    pub size: u32,
    pub structure: Structure,
    pub snippet: String,
}

fn text(value: Option<&[u8]>) -> Option<String> {
    HeaderForm::Text
        .parse(value?)
        .value()
        .as_text()
        .map(str::to_owned)
}

fn contacts<'a>(addresses: &'a Option<Vec<Address<'_>>>) -> impl Iterator<Item = Contact> + 'a {
    addresses.iter().flatten().filter_map(|a| {
        // Les délimiteurs de groupe IMAP n'ont pas de domaine.
        let local = std::str::from_utf8(a.mailbox.as_deref()?).ok()?.trim();
        let domain = std::str::from_utf8(a.host.as_deref()?).ok()?.trim();
        if local.is_empty() || domain.is_empty() {
            return None;
        }
        Some(Contact {
            address: format!("{local}@{}", domain.to_ascii_lowercase()),
            name: text(a.name.as_deref()),
        })
    })
}

fn ids(raw: Option<&[u8]>) -> Vec<String> {
    HeaderForm::MessageIds
        .parse(raw.unwrap_or_default())
        .value()
        .as_text_list()
        .into_iter()
        .flat_map(|list| list.iter().map(str::to_owned))
        .collect()
}

#[derive(Default)]
struct Parts<'a> {
    uid: Option<u32>,
    envelope: Option<&'a Envelope<'a>>,
    size: Option<u32>,
    date: Option<&'a str>,
    flags: Option<Vec<String>>,
    structure: Option<Structure>,
    header: &'a [u8],
}

impl<'a> Parts<'a> {
    fn collect(&mut self, attribute: &'a AttributeValue<'a>) {
        match attribute {
            AttributeValue::Uid(uid) => self.uid = Some(*uid),
            AttributeValue::Envelope(env) => self.envelope = Some(env),
            AttributeValue::Rfc822Size(size) => self.size = Some(*size),
            AttributeValue::InternalDate(date) => self.date = Some(date),
            AttributeValue::Flags(flags) => {
                self.flags = Some(flags.iter().map(ToString::to_string).collect())
            }
            AttributeValue::BodyStructure(body) => self.structure = Some(Structure::read(body)),
            AttributeValue::BodySection {
                section: Some(SectionPath::Full(MessageSection::Header)),
                data: Some(data),
                ..
            } => self.header = data,
            _ => {}
        }
    }
}

pub(super) fn parse(attributes: &[AttributeValue<'_>]) -> Result<Option<Header>, Error> {
    let mut parts = Parts::default();
    for attr in attributes {
        parts.collect(attr);
    }
    let Some(env) = parts.envelope else {
        // Le serveur peut annoncer spontanément un changement de drapeaux.
        return Ok(None);
    };

    let parsed = MessageParser::new().parse_headers(parts.header);
    let mut references: Vec<_> = parsed
        .as_ref()
        .and_then(|p| p.references())
        .into_iter()
        .flat_map(|r| r.iter().map(str::to_owned))
        .collect();
    references.extend(ids(env.in_reply_to.as_deref()));

    Ok(Some(Header {
        uid: parts.uid.ok_or("UID du message absent.")?,
        message_id: ids(env.message_id.as_deref()).into_iter().next(),
        references,
        sender: contacts(&env.from)
            .next()
            .or_else(|| contacts(&env.sender).next()),
        recipients: Recipients {
            to: contacts(&env.to).collect(),
            cc: contacts(&env.cc).collect(),
            bcc: contacts(&env.bcc).collect(),
        },
        subject: text(env.subject.as_deref()).unwrap_or_default(),
        sent_at: env
            .date
            .as_deref()
            .and_then(|date| HeaderForm::Date.parse(date).value().as_datetime())
            .map(|d| d.to_timestamp()),
        received_at: DateTime::parse_from_str(
            parts.date.ok_or("INTERNALDATE absent.")?.trim(),
            "%d-%b-%Y %H:%M:%S %z",
        )?
        .timestamp(),
        flags: parts.flags.ok_or("FLAGS absents.")?,
        size: parts.size.ok_or("Taille absente.")?,
        structure: parts.structure.ok_or("Structure MIME absente.")?,
        snippet: String::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn keeps_real_mailboxes_and_ignores_group_markers_and_incomplete_addresses() {
        let addresses = [
            (Some("Groupe"), None),
            (Some(""), Some("exemple.test")),
            (Some("SansDomaine"), Some("")),
            (Some("Anne"), Some(" EXEMPLE.TEST ")),
        ]
        .map(|(mailbox, host)| Address {
            name: None,
            adl: None,
            mailbox: mailbox.map(|s| Cow::Borrowed(s.as_bytes())),
            host: host.map(|s| Cow::Borrowed(s.as_bytes())),
        });
        let names: Vec<_> = contacts(&Some(Vec::from(addresses)))
            .map(|c| c.address)
            .collect();
        assert_eq!(names, ["Anne@exemple.test"]);
    }
}
