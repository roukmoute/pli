use std::{env, num::NonZeroU32};

use hex::FromHex;
use ring::{
    aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey},
    digest::{self, Digest},
    rand::{SecureRandom, SystemRandom},
};

use crate::Error;

pub(crate) struct SessionConfig {
    pub key: Option<LessSafeKey>,
    pub ttl: NonZeroU32,
}

impl SessionConfig {
    pub(crate) fn from_env() -> Result<Self, Error> {
        let key = env::var("WEBMAIL_SECRET_KEY").ok();
        let ttl = env::var("WEBMAIL_SESSION_TTL_SECONDS").unwrap_or_else(|_| "28800".into());
        Self::parse(key.as_deref(), &ttl)
    }

    pub(crate) fn parse(key: Option<&str>, ttl: &str) -> Result<Self, Error> {
        let key = key
            .filter(|value| !value.is_empty())
            .map(|value| {
                let bytes = <[u8; 32]>::from_hex(value)
                    .map_err(|_| "WEBMAIL_SECRET_KEY doit contenir 64 chiffres hexadécimaux.")?;
                UnboundKey::new(&AES_256_GCM, &bytes)
                    .map(LessSafeKey::new)
                    .map_err(|_| "Clé de chiffrement invalide.")
            })
            .transpose()?;
        Ok(Self {
            key,
            ttl: ttl.parse().map_err(|_| "WEBMAIL_SESSION_TTL_SECONDS doit être un entier strictement positif sur 32 bits.")?,
        })
    }
}

fn random_bytes<const N: usize>() -> Result<[u8; N], Error> {
    let mut bytes = [0; N];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "Aléa système indisponible.")?;
    Ok(bytes)
}

pub(super) fn encrypt(
    key: &LessSafeKey,
    profile: i64,
    address: &str,
    secret: String,
) -> Result<Vec<u8>, Error> {
    let nonce = random_bytes()?;
    let mut ciphertext = secret.into_bytes();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(format!("{profile}:{address}")),
        &mut ciphertext,
    )
    .map_err(|_| "Chiffrement indisponible.")?;
    let mut stored = nonce.to_vec();
    stored.append(&mut ciphertext);
    Ok(stored)
}

pub(crate) fn decrypt(
    key: &LessSafeKey,
    profile: i64,
    address: &str,
    stored: &mut [u8],
) -> Result<String, Error> {
    let (nonce, ciphertext) = stored
        .split_at_mut_checked(ring::aead::NONCE_LEN)
        .ok_or("Secret de messagerie incomplet.")?;
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| "Nonce invalide.")?;
    let plain = key
        .open_in_place(nonce, Aad::from(format!("{profile}:{address}")), ciphertext)
        .map_err(|_| "Secret de messagerie illisible.")?;
    Ok(std::str::from_utf8(plain)?.to_owned())
}

pub(super) struct Token([u8; 32]);

impl Token {
    pub fn generate() -> Result<Self, Error> {
        Ok(Self(random_bytes()?))
    }

    pub fn parse(value: &str) -> Option<Self> {
        <[u8; 32]>::from_hex(value).ok().map(Self)
    }

    pub fn encode(&self) -> String {
        hex::encode(self.0)
    }

    pub fn hash(&self) -> Digest {
        digest::digest(&digest::SHA256, &self.0)
    }
}
