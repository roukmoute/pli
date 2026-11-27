use std::num::NonZeroU32;

use async_imap::{
    imap_proto::{MailboxDatum, MailboxListData, Response},
    types::{Capability, NameAttribute},
};
use tokio::time::timeout;

use crate::{Error, admin::ImapSession};

use super::imap::{COMMAND_TIMEOUT, command};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Uids {
    pub validity: NonZeroU32,
    pub next: NonZeroU32,
}

pub(super) struct Folder {
    pub remote_path: String,
    pub delimiter: Option<String>,
    pub role: Option<&'static str>,
    pub uids: Option<Uids>,
}

pub(super) struct Discovery {
    pub inbox: Uids,
    pub folders: Vec<Folder>,
}

pub(super) fn display_path(remote_path: &str) -> String {
    encodify::utf7::IMAP
        .lenient()
        .decode(remote_path)
        .unwrap_or_else(|_| remote_path.to_owned())
}

fn special_role(attribute: &NameAttribute<'_>) -> Option<&'static str> {
    match attribute {
        NameAttribute::Sent => Some("sent"),
        NameAttribute::Drafts => Some("drafts"),
        NameAttribute::Junk => Some("junk"),
        NameAttribute::Trash => Some("trash"),
        NameAttribute::Archive => Some("archive"),
        _ => None,
    }
}

fn role_by_name(path: &str, delimiter: Option<&str>) -> Option<&'static str> {
    // Certains serveurs rangent les dossiers de la boîte directement sous INBOX.
    let label = match delimiter.and_then(|sep| path.rsplit_once(sep)) {
        Some((parent, label)) if parent.eq_ignore_ascii_case("INBOX") => label,
        Some(_) => return None,
        None => path,
    };
    match label.to_lowercase().as_str() {
        "sent"
        | "sent items"
        | "sent mail"
        | "envoyés"
        | "éléments envoyés"
        | "gesendet"
        | "gesendete elemente" => Some("sent"),
        "drafts" | "brouillons" | "entwürfe" => Some("drafts"),
        "junk" | "junk e-mail" | "spam" | "indésirables" | "courrier indésirable" => Some("junk"),
        "trash" | "deleted items" | "corbeille" | "papierkorb" | "gelöschte elemente" => {
            Some("trash")
        }
        "archive" | "archives" | "archiv" => Some("archive"),
        _ => None,
    }
}

fn archive_sans_attribut(names: &[MailboxListData<'_>]) -> Option<usize> {
    if names
        .iter()
        .any(|n| n.name_attributes.contains(&NameAttribute::Archive))
    {
        return None;
    }
    let mut candidats = names.iter().enumerate().filter(|(_, n)| {
        role_by_name(&display_path(&n.name), n.delimiter.as_deref()) == Some("archive")
            && n.name_attributes.iter().all(|attribut| match attribut {
                NameAttribute::NoInferiors | NameAttribute::Marked | NameAttribute::Unmarked => {
                    true
                }
                NameAttribute::Extension(valeur) => {
                    [r"\HasChildren", r"\HasNoChildren", r"\Subscribed"]
                        .iter()
                        .any(|nom| valeur.eq_ignore_ascii_case(nom))
                }
                _ => false,
            })
    });
    let (index, _) = candidats.next()?;
    candidats.next().is_none().then_some(index)
}

pub(super) async fn status(session: &mut ImapSession, name: &str) -> Result<Uids, Error> {
    let status = timeout(
        COMMAND_TIMEOUT,
        session.status(name, "(UIDVALIDITY UIDNEXT)"),
    )
    .await??;
    Ok(Uids {
        validity: status
            .uid_validity
            .and_then(NonZeroU32::new)
            .ok_or("UIDVALIDITY absent ou nul.")?,
        next: status
            .uid_next
            .and_then(NonZeroU32::new)
            .ok_or("UIDNEXT absent ou nul.")?,
    })
}

async fn list(
    session: &mut ImapSession,
    special_use: bool,
) -> Result<Vec<MailboxListData<'static>>, Error> {
    let list = if special_use {
        r#"LIST "" "*" RETURN (SPECIAL-USE)"#
    } else {
        r#"LIST "" "*""#
    };
    let mut listed = Vec::new();
    command(session, list, |response| {
        if let Response::MailboxData(MailboxDatum::List(name)) = response {
            listed.push(name.clone().into_owned());
        }
        Ok(())
    })
    .await?;
    Ok(listed)
}

pub(super) async fn discover(session: &mut ImapSession) -> Result<Discovery, Error> {
    // Une arrivée pendant le parcours des autres dossiers appartient déjà au nouveau courrier.
    let inbox = status(session, "INBOX").await?;
    let capabilities = timeout(COMMAND_TIMEOUT, session.capabilities()).await??;
    let special_use = capabilities
        .iter()
        .any(|c| matches!(c, Capability::Atom(name) if name.eq_ignore_ascii_case("SPECIAL-USE")));
    let names = list(session, special_use).await?;
    let archive = if special_use {
        archive_sans_attribut(&names)
    } else {
        None
    };
    let mut folders = Vec::with_capacity(names.len());
    for (index, name) in names.into_iter().enumerate() {
        let (role, uids) = if name.name_attributes.contains(&NameAttribute::NoSelect) {
            (None, None)
        } else if name.name.eq_ignore_ascii_case("INBOX") {
            (Some("inbox"), Some(inbox))
        } else {
            let role = if special_use {
                name.name_attributes
                    .iter()
                    .find_map(special_role)
                    .or_else(|| (archive == Some(index)).then_some("archive"))
            } else {
                role_by_name(&display_path(&name.name), name.delimiter.as_deref())
            };
            (role, Some(status(session, &name.name).await?))
        };
        folders.push(Folder {
            remote_path: name.name.into_owned(),
            delimiter: name.delimiter.map(|s| s.into_owned()),
            role,
            uids,
        });
    }
    if !folders.iter().any(|folder| folder.role == Some("inbox")) {
        return Err("INBOX absente de la liste des dossiers.".into());
    }
    Ok(Discovery { inbox, folders })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_seulement_une_archive_standard_non_attribuee() {
        let dossier = |name: &'static str, name_attributes| MailboxListData {
            name: name.into(),
            delimiter: Some("/".into()),
            name_attributes,
        };
        let archive = || {
            dossier(
                "INBOX/Archives",
                vec![NameAttribute::Extension(r"\HasNoChildren".into())],
            )
        };
        assert_eq!(
            archive_sans_attribut(&[dossier("Personnel/Archives", vec![]), archive()]),
            Some(1)
        );
        assert_eq!(
            archive_sans_attribut(&[archive(), dossier("Archive", vec![])]),
            None
        );
        assert_eq!(
            archive_sans_attribut(&[archive(), dossier("Classés", vec![NameAttribute::Archive])]),
            None
        );
        for attribut in [
            NameAttribute::NoSelect,
            NameAttribute::All,
            NameAttribute::Trash,
            NameAttribute::Extension(r"\Important".into()),
        ] {
            assert_eq!(
                archive_sans_attribut(&[dossier("Archives", vec![attribut])]),
                None
            );
        }
        for attribut in [
            NameAttribute::Marked,
            NameAttribute::Unmarked,
            NameAttribute::NoInferiors,
            NameAttribute::Extension(r"\subscribed".into()),
            NameAttribute::Extension(r"\HasChildren".into()),
        ] {
            assert_eq!(
                archive_sans_attribut(&[dossier("Archiv", vec![attribut])]),
                Some(0)
            );
        }
        assert_eq!(
            archive_sans_attribut(&[dossier("Personnel/Archives", vec![])]),
            None
        );
    }

    #[test]
    fn matches_the_six_roles_or_keeps_a_personal_folder() {
        for (attribute, role) in [
            (NameAttribute::Sent, "sent"),
            (NameAttribute::Drafts, "drafts"),
            (NameAttribute::Junk, "junk"),
            (NameAttribute::Trash, "trash"),
            (NameAttribute::Archive, "archive"),
        ] {
            assert_eq!(special_role(&attribute), Some(role));
        }
        assert_eq!(special_role(&NameAttribute::All), None);
        for (path, role) in [
            ("INBOX.Gesendet", "sent"),
            ("Envoyés", "sent"),
            ("Brouillons", "drafts"),
            ("Entwürfe", "drafts"),
            ("Indésirables", "junk"),
            ("PAPIERKORB", "trash"),
            ("Archives", "archive"),
        ] {
            assert_eq!(role_by_name(path, Some(".")), Some(role));
        }
        assert_eq!(role_by_name("Archives/Envoyés", Some("/")), None);
        assert_eq!(role_by_name("INBOX.Sent", None), None);
    }
}
