//! Signed-QR scheme [M §7.1.6, §10.5]. Ed25519 (blueprint §2 #3, §6.1): the
//! server signs; gate devices verify offline with only the public key, so a
//! stolen scanner cannot mint tickets.

use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Compact signed payload embedded in a ticket/invitation QR.
#[derive(Debug, Serialize, Deserialize)]
pub struct QrPayload {
    pub v: u8,            // scheme version
    pub tid: String,      // ticket id
    pub eid: String,      // event id
    pub zid: String,      // zone id
    pub typ: String,      // ticket type
    pub iat: i64,         // issued-at (unix)
    pub nonce: String,    // anti-replay salt
}

#[derive(Clone)]
pub struct QrSigner {
    key: SigningKey,
}

impl QrSigner {
    /// Build from a base64 (no-pad) 32-byte seed, or generate a random key.
    /// In production set QR_SIGNING_SEED so restarts keep the same key.
    pub fn from_seed_or_random(seed_b64: Option<&str>) -> Self {
        if let Some(s) = seed_b64 {
            if let Ok(bytes) = STANDARD_NO_PAD.decode(s) {
                if bytes.len() == 32 {
                    let mut seed = [0u8; 32];
                    seed.copy_from_slice(&bytes);
                    return Self { key: SigningKey::from_bytes(&seed) };
                }
            }
            tracing::warn!("QR_SIGNING_SEED invalid; generating an ephemeral key");
        }
        let mut csprng = rand::rngs::OsRng;
        Self { key: SigningKey::generate(&mut csprng) }
    }

    /// base64(no-pad) payload "." base64(no-pad) signature.
    pub fn sign(&self, payload: &QrPayload) -> String {
        let json = serde_json::to_vec(payload).expect("serialize qr payload");
        let payload_b64 = STANDARD_NO_PAD.encode(&json);
        let sig: Signature = self.key.sign(payload_b64.as_bytes());
        let sig_b64 = STANDARD_NO_PAD.encode(sig.to_bytes());
        format!("{payload_b64}.{sig_b64}")
    }

    pub fn public_key_b64(&self) -> String {
        STANDARD_NO_PAD.encode(self.key.verifying_key().to_bytes())
    }
}

/// Offline verification helper (mirrors what the Android app does with the
/// public key from GET /access/public-key). Returns the decoded payload.
pub fn verify(public_key_b64: &str, token: &str) -> Option<QrPayload> {
    let (payload_b64, sig_b64) = token.split_once('.')?;
    let pk_bytes = STANDARD_NO_PAD.decode(public_key_b64).ok()?;
    let vk = VerifyingKey::from_bytes(pk_bytes.as_slice().try_into().ok()?).ok()?;
    let sig_bytes = STANDARD_NO_PAD.decode(sig_b64).ok()?;
    let sig = Signature::from_slice(&sig_bytes).ok()?;
    vk.verify(payload_b64.as_bytes(), &sig).ok()?;
    let json = STANDARD_NO_PAD.decode(payload_b64).ok()?;
    serde_json::from_slice(&json).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> QrPayload {
        QrPayload { v: 1, tid: "t-1".into(), eid: "e-1".into(), zid: "Z1".into(),
            typ: "local".into(), iat: 1_730_000_000, nonce: "n".into() }
    }

    #[test]
    fn sign_then_verify_roundtrips() {
        let s = QrSigner::from_seed_or_random(None);
        let token = s.sign(&payload());
        let got = verify(&s.public_key_b64(), &token).expect("valid token verifies");
        assert_eq!(got.tid, "t-1");
        assert_eq!(got.zid, "Z1");
    }

    #[test]
    fn tampered_payload_is_rejected() {
        let s = QrSigner::from_seed_or_random(None);
        let token = s.sign(&payload());
        let (p, sig) = token.split_once('.').unwrap();
        // flip one character of the payload
        let mut bad = p.to_string();
        bad.replace_range(0..1, if p.starts_with('A') { "B" } else { "A" });
        let forged = format!("{bad}.{sig}");
        assert!(verify(&s.public_key_b64(), &forged).is_none());
    }

    #[test]
    fn wrong_key_is_rejected() {
        let a = QrSigner::from_seed_or_random(None);
        let b = QrSigner::from_seed_or_random(None);
        let token = a.sign(&payload());
        assert!(verify(&b.public_key_b64(), &token).is_none());
    }

    #[test]
    fn seed_is_deterministic() {
        let seed = Some("Io5U1Pz42A1lJj6FGN8tXeWtuLoJvF6iiuTaEL9bWJ4");
        let a = QrSigner::from_seed_or_random(seed);
        let b = QrSigner::from_seed_or_random(seed);
        assert_eq!(a.public_key_b64(), b.public_key_b64());
    }
}
