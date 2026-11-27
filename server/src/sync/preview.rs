use std::collections::BTreeMap;

use async_imap::imap_proto::{
    AttributeValue, BodyContentCommon, BodyStructure, ContentEncoding, Response, SectionPath,
};
use mail_parser::{
    Encoding,
    decoders::{
        charsets,
        preview::{preview_html, preview_text},
    },
};

use super::{header::Header, imap::command};
use crate::{Error, admin::ImapSession};

const PREFIX_BYTES: u32 = 4096;

pub(super) struct Part {
    path: Vec<u32>,
    octets: u32,
    html: bool,
    charset: Vec<u8>,
    encoding: Encoding,
}

impl Part {
    fn item(&self, local_test: bool) -> String {
        let section = self
            .path
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".");
        let mut item = format!("BODY.PEEK[{section}]");
        // GreenMail 2.1.15 omet l'espace obligatoire avant le littéral des réponses
        // partielles. Sur ce profil de banc, lire la partie textuelle sélectionnée.
        if !local_test && self.octets > PREFIX_BYTES {
            item.push_str(&format!("<0.{PREFIX_BYTES}>"));
        }
        item
    }

    fn snippet(&self, bytes: &[u8]) -> String {
        let decoded = self.encoding.decode(bytes);
        let text = charsets::decode(&self.charset, &decoded);
        let preview = if self.html {
            preview_html(&text, 200)
        } else {
            preview_text(&text, 200).into_owned()
        };
        preview.trim().to_owned()
    }
}

fn named(common: &BodyContentCommon<'_>) -> bool {
    common
        .ty
        .params
        .iter()
        .flatten()
        .any(|(key, _)| key.eq_ignore_ascii_case("name"))
        || common.disposition.as_ref().is_some_and(|d| {
            d.ty.eq_ignore_ascii_case("attachment")
                || d.params
                    .iter()
                    .flatten()
                    .any(|(key, _)| key.eq_ignore_ascii_case("filename"))
        })
}

#[derive(Default)]
pub(super) struct Structure {
    pub attached: bool,
    pub part: Option<Part>,
}

impl Structure {
    pub fn read(body: &BodyStructure<'_>) -> Self {
        let mut structure = Self::default();
        structure.visit(body, &mut Vec::new());
        structure
    }

    fn visit(&mut self, body: &BodyStructure<'_>, path: &mut Vec<u32>) {
        match body {
            BodyStructure::Multipart { common, bodies, .. } if !named(common) => {
                for (index, child) in bodies.iter().enumerate() {
                    path.push((index + 1) as u32);
                    self.visit(child, path);
                    path.pop();
                }
            }
            BodyStructure::Text { common, other, .. } if !named(common) => {
                let html = common.ty.subtype.eq_ignore_ascii_case("html");
                if !html && !common.ty.subtype.eq_ignore_ascii_case("plain") {
                    return;
                }
                if self.part.as_ref().is_some_and(|part| !part.html || html) {
                    return;
                }
                let charset = common
                    .ty
                    .params
                    .iter()
                    .flatten()
                    .find(|(key, _)| key.eq_ignore_ascii_case("charset"))
                    .map(|(_, v)| v.as_bytes())
                    .unwrap_or_default();
                self.part = Some(Part {
                    path: if path.is_empty() {
                        vec![1]
                    } else {
                        path.clone()
                    },
                    octets: other.octets,
                    html,
                    charset: charset.to_vec(),
                    encoding: match other.transfer_encoding {
                        ContentEncoding::Base64 => Encoding::Base64,
                        ContentEncoding::QuotedPrintable => Encoding::QuotedPrintable,
                        _ => Encoding::None,
                    },
                });
            }
            _ => self.attached = true,
        }
    }
}

fn body<'a>(attrs: &'a [AttributeValue<'_>]) -> Option<(u32, &'a [u32], &'a [u8])> {
    let mut uid = None;
    let mut section = None;
    for attr in attrs {
        match attr {
            AttributeValue::Uid(id) => uid = Some(*id),
            AttributeValue::BodySection {
                section: Some(SectionPath::Part(path, None)),
                data: Some(data),
                ..
            } => {
                section = Some((path.as_slice(), data.as_ref()));
            }
            _ => {}
        }
    }
    let (path, bytes) = section?;
    Some((uid?, path, bytes))
}

// Les lettres qui ont la même partie textuelle sont lues dans un seul aller-retour.
// Une pièce binaire ne sert jamais à produire un extrait.
pub(super) async fn load(
    session: &mut ImapSession,
    messages: &mut [Header],
    local_test: bool,
) -> Result<(), Error> {
    let mut groups = BTreeMap::<String, Vec<String>>::new();
    for header in messages.iter() {
        if let Some(part) = &header.structure.part {
            groups
                .entry(part.item(local_test))
                .or_default()
                .push(header.uid.to_string());
        }
    }
    for (item, ids) in groups {
        command(
            session,
            &format!("UID FETCH {} (UID {item})", ids.join(",")),
            |response| {
                if let Response::Fetch(_, attrs) = response
                    && let Some((uid, path, data)) = body(attrs)
                    && let Some(message) = messages.iter_mut().find(|m| m.uid == uid)
                    && let Some(part) = &message.structure.part
                    && part.path == path
                {
                    message.snippet = part.snippet(data);
                }
                Ok(())
            },
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure(body: &str) -> Structure {
        let raw = format!("* 1 FETCH (BODYSTRUCTURE {body})\r\n");
        let (_, Response::Fetch(_, attrs)) = Response::parse(raw.as_bytes()).unwrap() else {
            panic!("FETCH attendu")
        };
        let AttributeValue::BodyStructure(body) = &attrs[0] else {
            panic!("BODYSTRUCTURE attendue")
        };
        Structure::read(body)
    }

    #[test]
    fn selects_the_first_plain_text_even_when_nested_and_decodes_only_its_preview() {
        let html = r#"("text" "HTML" NIL NIL NIL "base64" 4096 1)"#;
        let plain =
            r#"("text" "PLAIN" ("charset" "iso-8859-1") NIL NIL "quoted-printable" 4097 1)"#;
        let alternative = format!("({html}{plain}{html}{plain} \"ALTERNATIVE\")");
        for attachment in [
            r#"("application" "pdf" NIL NIL NIL "base64" 20000)"#,
            r#"("text" "plain" ("name" "rapport") NIL NIL "7bit" 56 1)"#,
            r#"("text" "plain" NIL NIL NIL "7bit" 56 1 NIL ("attachment" NIL))"#,
            r#"("text" "plain" NIL NIL NIL "7bit" 56 1 NIL ("inline" ("filename" "rapport")))"#,
        ] {
            let mime = structure(&format!("({attachment}{alternative} \"MIXED\")"));
            assert!(mime.attached);
            let part = mime.part.unwrap();
            assert_eq!(part.item(false), "BODY.PEEK[2.2]<0.4096>");
            assert_eq!(part.item(true), "BODY.PEEK[2.2]");
            assert_eq!(part.snippet(b"=E9t=E9"), "été");
        }
        let single = structure(html);
        assert!(!single.attached);
        let part = single.part.unwrap();
        assert_eq!(part.item(false), "BODY.PEEK[1]");
        assert_eq!(part.snippet(b"PHA+Qm9uam91cjwvcD4="), "Bonjour");
        assert!(
            structure(r#"("text" "calendar" NIL NIL NIL "7bit" 5 1)"#)
                .part
                .is_none()
        );
    }
}
