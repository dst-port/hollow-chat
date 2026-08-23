use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;
use rand::Rng;

use crate::error::AppError;

const PASSWORD_ALPHABET: &[u8] =
    b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+";
const PASSWORD_LENGTH: usize = 28;

pub fn generate_password() -> String {
    let mut rng = OsRng;
    (0..PASSWORD_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..PASSWORD_ALPHABET.len());
            PASSWORD_ALPHABET[idx] as char
        })
        .collect()
}

fn argon2_with_pepper(pepper: &[u8]) -> Argon2<'_> {
    Argon2::new_with_secret(
        pepper,
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::default(),
    )
    .expect("valid argon2 pepper")
}

pub fn hash_password(password: &str, pepper: &[u8]) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = argon2_with_pepper(pepper);
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::Hashing)
}

pub fn verify_password(password: &str, hash: &str, pepper: &[u8]) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| AppError::Hashing)?;
    let argon2 = argon2_with_pepper(pepper);
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
