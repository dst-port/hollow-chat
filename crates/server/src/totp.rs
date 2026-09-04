use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::RngCore;
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

const SECRET_BYTES: usize = 20;
const STEP_SECONDS: u64 = 30;
const DIGITS: u32 = 6;
const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn generate_secret() -> String {
    let mut bytes = [0u8; SECRET_BYTES];
    OsRng.fill_bytes(&mut bytes);
    base32_encode(&bytes)
}

fn base32_encode(data: &[u8]) -> String {
    let mut output = String::new();
    let mut buffer: u32 = 0;
    let mut bits_left = 0u32;

    for &byte in data {
        buffer = (buffer << 8) | byte as u32;
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            output.push(BASE32_ALPHABET[((buffer >> bits_left) & 0x1f) as usize] as char);
        }
    }
    if bits_left > 0 {
        output.push(BASE32_ALPHABET[((buffer << (5 - bits_left)) & 0x1f) as usize] as char);
    }

    output
}

fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let mut buffer: u32 = 0;
    let mut bits_left = 0u32;
    let mut output = Vec::new();

    for c in input.trim().to_uppercase().chars() {
        if c == '=' {
            continue;
        }
        let value = BASE32_ALPHABET.iter().position(|&b| b as char == c)? as u32;
        buffer = (buffer << 5) | value;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            output.push(((buffer >> bits_left) & 0xff) as u8);
        }
    }

    Some(output)
}

fn hotp(secret: &[u8], counter: u64) -> u32 {
    let mut mac = HmacSha1::new_from_slice(secret).expect("hmac accepts keys of any length");
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let offset = (result[result.len() - 1] & 0x0f) as usize;
    let code = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);

    code % 10u32.pow(DIGITS)
}

/// Which time step a submitted code matches, if any.
///
/// Callers need the step, not just a yes/no: a TOTP code is valid for a ~90
/// second window, so without recording the step that was used, the same code
/// can be redeemed over and over inside that window (RFC 6238 §5.2). Compare
/// the returned step against the last one accepted for that user and reject
/// anything that isn't newer.
pub fn matching_counter(secret_base32: &str, code: &str) -> Option<u64> {
    if code.len() != DIGITS as usize || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let secret = base32_decode(secret_base32)?;
    let submitted = code.parse::<u32>().ok()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let counter = now / STEP_SECONDS;

    for drift in [-1i64, 0, 1] {
        let step = (counter as i64 + drift).max(0) as u64;
        if hotp(&secret, step) == submitted {
            return Some(step);
        }
    }

    None
}

pub fn otpauth_url(secret_base32: &str, username: &str, issuer: &str) -> String {
    format!(
        "otpauth://totp/{issuer}:{username}?secret={secret_base32}&issuer={issuer}&digits={DIGITS}&period={STEP_SECONDS}"
    )
}
