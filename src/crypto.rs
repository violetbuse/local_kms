use std::collections::BTreeMap;

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng, Payload},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::error::AppError;

pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
const BLOB_VERSION: u8 = 1;

pub struct DecodedBlob {
    pub key_id: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Fills a buffer of `len` cryptographically random bytes.
pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    OsRng.fill_bytes(&mut buf);
    buf
}

/// A random 64-char lowercase hex identifier, matching KMS's `KeyMaterialId` shape.
pub fn hex_id() -> String {
    random_bytes(32).iter().map(|b| format!("{b:02x}")).collect()
}

/// Canonical AAD encoding for an EncryptionContext map: sorted by key (BTreeMap iteration
/// order), each entry length-prefixed to avoid concatenation collisions between maps like
/// {"a":"bc"} and {"ab":"c"}.
pub fn canonicalize_aad(context: &BTreeMap<String, String>) -> Vec<u8> {
    let mut aad = Vec::new();
    for (k, v) in context {
        aad.extend_from_slice(&(k.len() as u32).to_be_bytes());
        aad.extend_from_slice(k.as_bytes());
        aad.extend_from_slice(&(v.len() as u32).to_be_bytes());
        aad.extend_from_slice(v.as_bytes());
    }
    aad
}

pub fn encrypt(key: &[u8], key_id: &str, plaintext: &[u8], aad: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, Payload { msg: plaintext, aad })
        .expect("AES-GCM encryption cannot fail for valid key/nonce sizes");

    encode_blob(key_id, &nonce_bytes, &ciphertext)
}

pub fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, AppError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce);

    cipher
        .decrypt(nonce, Payload { msg: ciphertext, aad })
        .map_err(|_| AppError::InvalidCiphertext("ciphertext could not be authenticated".into()))
}

pub fn encode_blob(key_id: &str, nonce: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let key_id_bytes = key_id.as_bytes();
    let mut blob = Vec::with_capacity(1 + 2 + key_id_bytes.len() + nonce.len() + ciphertext.len());
    blob.push(BLOB_VERSION);
    blob.extend_from_slice(&(key_id_bytes.len() as u16).to_be_bytes());
    blob.extend_from_slice(key_id_bytes);
    blob.extend_from_slice(nonce);
    blob.extend_from_slice(ciphertext);
    blob
}

pub fn decode_blob(bytes: &[u8]) -> Result<DecodedBlob, AppError> {
    let malformed = || AppError::InvalidCiphertext("malformed ciphertext blob".into());

    if bytes.len() < 3 {
        return Err(malformed());
    }
    if bytes[0] != BLOB_VERSION {
        return Err(malformed());
    }
    let key_id_len = u16::from_be_bytes([bytes[1], bytes[2]]) as usize;
    let key_id_start: usize = 3;
    let key_id_end = key_id_start
        .checked_add(key_id_len)
        .ok_or_else(malformed)?;

    if bytes.len() < key_id_end {
        return Err(malformed());
    }
    let key_id = String::from_utf8(bytes[key_id_start..key_id_end].to_vec())
        .map_err(|_| malformed())?;

    let nonce_start = key_id_end;
    let nonce_end = nonce_start + NONCE_LEN;
    if bytes.len() < nonce_end + TAG_LEN {
        return Err(malformed());
    }
    let nonce = bytes[nonce_start..nonce_end].to_vec();
    let ciphertext = bytes[nonce_end..].to_vec();

    Ok(DecodedBlob { key_id, nonce, ciphertext })
}

pub fn base64_encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn base64_decode(s: &str) -> Result<Vec<u8>, AppError> {
    STANDARD
        .decode(s)
        .map_err(|_| AppError::InvalidCiphertext("CiphertextBlob is not valid base64".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn aad_is_deterministic() {
        let a = canonicalize_aad(&ctx(&[("purpose", "test"), ("env", "dev")]));
        let b = canonicalize_aad(&ctx(&[("env", "dev"), ("purpose", "test")]));
        assert_eq!(a, b);
    }

    #[test]
    fn aad_avoids_concatenation_collisions() {
        let a = canonicalize_aad(&ctx(&[("a", "bc")]));
        let b = canonicalize_aad(&ctx(&[("ab", "c")]));
        assert_ne!(a, b);
    }

    #[test]
    fn blob_round_trips() {
        let key = random_bytes(32);
        let plaintext = b"hello world";
        let blob = encrypt(&key, "my-key-id", plaintext, b"");
        let decoded = decode_blob(&blob).expect("decode should succeed");
        assert_eq!(decoded.key_id, "my-key-id");
        let out = decrypt(&key, &decoded.nonce, &decoded.ciphertext, b"").expect("decrypt should succeed");
        assert_eq!(out, plaintext);
    }

    #[test]
    fn decode_rejects_truncated_blob() {
        assert!(decode_blob(&[1, 0]).is_err());
        assert!(decode_blob(&[1, 0, 5, b'h', b'i']).is_err());
    }

    #[test]
    fn encrypt_decrypt_round_trip_with_matching_aad() {
        let key = random_bytes(32);
        let aad = canonicalize_aad(&ctx(&[("k", "v")]));
        let blob = encrypt(&key, "kid", b"secret", &aad);
        let decoded = decode_blob(&blob).unwrap();
        let out = decrypt(&key, &decoded.nonce, &decoded.ciphertext, &aad).unwrap();
        assert_eq!(out, b"secret");
    }

    #[test]
    fn decrypt_fails_with_mismatched_aad() {
        let key = random_bytes(32);
        let aad = canonicalize_aad(&ctx(&[("k", "v")]));
        let other_aad = canonicalize_aad(&ctx(&[("k", "different")]));
        let blob = encrypt(&key, "kid", b"secret", &aad);
        let decoded = decode_blob(&blob).unwrap();
        assert!(decrypt(&key, &decoded.nonce, &decoded.ciphertext, &other_aad).is_err());
    }
}
