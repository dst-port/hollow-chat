// Encrypts small, high-entropy secrets (TOTP seeds) at rest, keyed off the
// same pepper already used for password hashing.
//
// This is a different threat model from the IP-address case we deliberately
// chose NOT to hash: an IP is low-entropy (the whole IPv4 space is brute-
// forceable in seconds), so hashing/encrypting it with a key that lives on
// the same host adds no real protection - an attacker with root has both.
// A TOTP secret is a random ~20-byte value with no meaningful keyspace to
// brute-force, so encrypting it is a genuine win against the more likely
// "DB dump leaked, app server/pepper weren't" scenario (SQL injection, a
// stray backup, a misconfigured replica) - it just doesn't help against a
// full root compromise where the pepper leaks alongside the database,
// which is the same irreducible limit noted for the pepper-colocation
// issue itself.
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

use crate::error::AppError;

const INFO: &[u8] = b"hollowchat-secret-box-v1";

fn derive_key(pepper: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, pepper);
    let mut key = [0u8; 32];
    hk.expand(INFO, &mut key)
        .expect("32 bytes is a valid HKDF output length");
    key
}

/// Encrypts `plaintext`, returning a base64 blob (nonce || ciphertext)
/// safe to store directly in a text column.
pub fn seal(pepper: &[u8], plaintext: &str) -> String {
    let key = derive_key(pepper);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key is always 32 bytes");
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .expect("AES-GCM encryption over a bounded, well-formed buffer cannot fail");

    let mut out = Vec::with_capacity(nonce.len() + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, out)
}

/// Reverses [`seal`]. Fails closed (`AppError::Internal`) on any malformed
/// or tampered input rather than guessing.
pub fn open(pepper: &[u8], sealed: &str) -> Result<String, AppError> {
    let raw = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, sealed)
        .map_err(|_| AppError::Internal)?;
    if raw.len() < 12 {
        return Err(AppError::Internal);
    }
    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let key = derive_key(pepper);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key is always 32 bytes");
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::Internal)?;
    String::from_utf8(plaintext).map_err(|_| AppError::Internal)
}
