//! Minimal Web Push (RFC 8291 `aes128gcm` + RFC 8292 VAPID) sender built on
//! the crates already in the tree - no `web-push`/curl dependency. Encrypts a
//! JSON payload for one browser subscription and POSTs it to the push service.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Key, Nonce};
use base64::Engine;
use hkdf::Hkdf;
use p256::ecdh::EphemeralSecret;
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::elliptic_curve::rand_core::{OsRng, RngCore};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use sha2::Sha256;

const URL_SAFE: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A parsed VAPID application key: the ES256 signer plus the base64url
/// uncompressed public point browsers pin as `applicationServerKey`.
pub struct VapidKey {
    signing: SigningKey,
    pub public_b64: String,
}

impl VapidKey {
    /// Parse an SEC1 `EC PRIVATE KEY` PEM (what `openssl ecparam -genkey` emits).
    pub fn from_sec1_pem(pem: &str) -> Result<Self, String> {
        let secret = SecretKey::from_sec1_pem(pem).map_err(|e| format!("bad VAPID key: {e}"))?;
        let public_b64 =
            URL_SAFE.encode(secret.public_key().to_encoded_point(false).as_bytes());
        Ok(Self {
            signing: SigningKey::from(&secret),
            public_b64,
        })
    }

    /// A signed VAPID JWT for one push-service origin (`aud`).
    fn jwt(&self, aud: &str, subject: &str) -> Result<String, String> {
        let header = URL_SAFE.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let exp = chrono::Utc::now().timestamp() + 12 * 60 * 60;
        let claims = serde_json::json!({ "aud": aud, "exp": exp, "sub": subject });
        let claims = URL_SAFE.encode(serde_json::to_vec(&claims).map_err(|e| e.to_string())?);
        let signing_input = format!("{header}.{claims}");
        let sig: Signature = self.signing.sign(signing_input.as_bytes());
        Ok(format!("{signing_input}.{}", URL_SAFE.encode(sig.to_bytes())))
    }
}

/// Tolerant base64url/base64 decode - browsers send the subscription keys
/// base64url-unpadded, but be forgiving about padding and alphabet.
fn b64_any(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim().replace('+', "-").replace('/', "_");
    let s = s.trim_end_matches('=');
    URL_SAFE.decode(s).map_err(|e| e.to_string())
}

fn origin_of(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(u) => match (u.scheme(), u.host_str()) {
            (scheme, Some(host)) => match u.port() {
                Some(port) => format!("{scheme}://{host}:{port}"),
                None => format!("{scheme}://{host}"),
            },
            _ => url.to_string(),
        },
        Err(_) => url.to_string(),
    }
}

/// Encrypt `plaintext` for a subscription's `p256dh`/`auth` keys, producing the
/// full `aes128gcm` request body (RFC 8188 header || ciphertext).
fn encrypt(p256dh: &[u8], auth: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let ua_public = PublicKey::from_sec1_bytes(p256dh).map_err(|e| e.to_string())?;
    let as_secret = EphemeralSecret::random(&mut OsRng);
    let as_point = as_secret.public_key().to_encoded_point(false);
    let as_public = as_point.as_bytes();
    let ua_point = ua_public.to_encoded_point(false);
    let ua_public_bytes = ua_point.as_bytes();

    let shared = as_secret.diffie_hellman(&ua_public);

    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);

    // RFC 8291 3.4: IKM = HKDF(auth, ecdh, "WebPush: info\0" || ua_pub || as_pub)
    let mut key_info = Vec::with_capacity(14 + 65 + 65);
    key_info.extend_from_slice(b"WebPush: info\0");
    key_info.extend_from_slice(ua_public_bytes);
    key_info.extend_from_slice(as_public);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth), shared.raw_secret_bytes().as_slice())
        .expand(&key_info, &mut ikm)
        .map_err(|e| e.to_string())?;

    // RFC 8188: derive the content key and nonce from the record salt.
    let hk = Hkdf::<Sha256>::new(Some(&salt), &ikm);
    let mut cek = [0u8; 16];
    hk.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .map_err(|e| e.to_string())?;
    let mut nonce = [0u8; 12];
    hk.expand(b"Content-Encoding: nonce\0", &mut nonce)
        .map_err(|e| e.to_string())?;

    // Single record: payload followed by the 0x02 "last record" delimiter.
    let mut padded = plaintext.to_vec();
    padded.push(0x02);

    let cipher = Aes128Gcm::new(Key::<Aes128Gcm>::from_slice(&cek));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), padded.as_ref())
        .map_err(|e| e.to_string())?;

    let mut body = Vec::with_capacity(21 + as_public.len() + ciphertext.len());
    body.extend_from_slice(&salt);
    body.extend_from_slice(&4096u32.to_be_bytes()); // record size
    body.push(as_public.len() as u8); // keyid length
    body.extend_from_slice(as_public); // keyid = our ephemeral public key
    body.extend_from_slice(&ciphertext);
    Ok(body)
}

#[derive(Debug)]
pub enum DeliveryError {
    /// The push service says this subscription is dead (404/410) - drop it.
    Gone,
    /// Anything else (network, 4xx/5xx) - keep the subscription, try later.
    Transient(String),
}

/// Encrypt and POST one notification. `p256dh`/`auth` are the base64 strings as
/// stored from the browser's `PushSubscription`.
pub async fn deliver(
    http: &reqwest::Client,
    vapid: &VapidKey,
    subject: &str,
    endpoint: &str,
    p256dh: &str,
    auth: &str,
    payload: &[u8],
) -> Result<(), DeliveryError> {
    let p256dh = b64_any(p256dh).map_err(DeliveryError::Transient)?;
    let auth = b64_any(auth).map_err(DeliveryError::Transient)?;
    let body = encrypt(&p256dh, &auth, payload).map_err(DeliveryError::Transient)?;
    let jwt = vapid
        .jwt(&origin_of(endpoint), subject)
        .map_err(DeliveryError::Transient)?;

    let resp = http
        .post(endpoint)
        .header("Content-Encoding", "aes128gcm")
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .header("TTL", "86400")
        .header("Urgency", "normal")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("vapid t={jwt}, k={}", vapid.public_b64),
        )
        .body(body)
        .send()
        .await
        .map_err(|e| DeliveryError::Transient(e.to_string()))?;

    match resp.status().as_u16() {
        200 | 201 | 202 | 204 => Ok(()),
        404 | 410 => Err(DeliveryError::Gone),
        code => Err(DeliveryError::Transient(format!("push service {code}"))),
    }
}
