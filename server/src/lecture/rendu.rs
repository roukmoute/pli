use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use ammonia::{Builder, Url, UrlRelative};
use mail_parser::{MessageParser, MessagePart};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};

use crate::Error;

// Invalider aussi les images lorsqu’une politique de rendu change.
pub(super) const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(super) enum Image {
    Integree { partie: u32 },
    Distante { url: String },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Piece {
    nom: Option<String>,
    mime: String,
    taille: usize,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Contenu {
    pub html: String,
    pub images: Vec<Image>,
    pub pieces: Vec<Piece>,
}

pub(super) struct ImageMime {
    pub partie: u32,
    pub mime: String,
    pub octets: Vec<u8>,
}

fn mime(part: MessagePart<'_>) -> String {
    part.content_type()
        .and_then(|t| Some(format!("{}/{}", t.ctype(), t.subtype()?).to_ascii_lowercase()))
        .unwrap_or_else(|| "application/octet-stream".into())
}

fn raster(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/gif" | "image/webp"
    )
}

fn reference(value: &str, parties: &HashMap<String, u32>) -> Option<Image> {
    let url = Url::parse(value.trim()).ok()?;
    match url.scheme() {
        "cid" => {
            let id = percent_decode_str(url.path()).decode_utf8().ok()?;
            Some(Image::Integree {
                partie: *parties.get(id.as_ref())?,
            })
        }
        "http" | "https" => Some(Image::Distante { url: url.into() }),
        _ => None,
    }
}

fn assainir(source: &str, parties: HashMap<String, u32>) -> (String, Vec<Image>) {
    let images = Arc::new(Mutex::new(Vec::new()));
    let capture = images.clone();
    let html = Builder::default()
        .tags(
            [
                "p",
                "div",
                "span",
                "br",
                "hr",
                "a",
                "img",
                "strong",
                "b",
                "em",
                "i",
                "u",
                "s",
                "small",
                "sub",
                "sup",
                "blockquote",
                "pre",
                "code",
                "h1",
                "h2",
                "h3",
                "h4",
                "h5",
                "h6",
                "ul",
                "ol",
                "li",
                "dl",
                "dt",
                "dd",
                "table",
                "thead",
                "tbody",
                "tfoot",
                "tr",
                "th",
                "td",
                "caption",
                "figure",
                "figcaption",
            ]
            .into_iter()
            .collect(),
        )
        .add_clean_content_tags(["form", "iframe", "object", "template", "svg", "math"])
        .generic_attributes(HashSet::from(["lang", "dir", "title"]))
        .tag_attributes(HashMap::from([
            ("a", HashSet::from(["href"])),
            ("img", HashSet::from(["src", "alt"])),
            ("td", HashSet::from(["colspan", "rowspan"])),
            ("th", HashSet::from(["colspan", "rowspan"])),
        ]))
        .url_schemes(HashSet::from(["http", "https", "mailto", "cid"]))
        .url_relative(UrlRelative::Deny)
        .attribute_filter(move |tag, attr, value| {
            if (tag, attr) != ("img", "src") {
                return Some(value.into());
            }
            let image = reference(value, &parties)?;
            let mut list = capture.lock().ok()?;
            let index = list.len();
            list.push(image);
            // Un identifiant opaque ne peut déclencher aucune requête HTTP.
            Some(format!("cid:pli-{index}").into())
        })
        .clean(source)
        .to_string();
    let images = std::mem::take(&mut *images.lock().expect("analyse HTML terminée"));
    (html, images)
}

pub(super) fn analyser(bytes: &[u8]) -> Result<(Contenu, Vec<ImageMime>), Error> {
    let message = MessageParser::default()
        .parse(bytes)
        .ok_or("Message MIME illisible.")?;
    let mut identifiants = HashMap::new();
    let mut pieces = Vec::new();
    let mut integrees = Vec::new();
    for part in message.root().parts() {
        let mime = mime(part);
        if part.is_attachment() {
            pieces.push(Piece {
                nom: part.attachment_name().map(str::to_owned),
                mime: mime.clone(),
                taille: part.decoded_len(),
            });
        }
        if let Some(cid) = part.content_id()
            && raster(&mime)
        {
            identifiants.insert(cid.to_owned(), part.id());
            integrees.push(ImageMime {
                partie: part.id(),
                mime,
                octets: part.decoded().into_owned(),
            });
        }
    }
    let source = message.html_bodies().collect::<Vec<_>>().join("\n");
    let (html, images) = assainir(&source, identifiants);
    Ok((
        Contenu {
            html,
            images,
            pieces,
        },
        integrees,
    ))
}
