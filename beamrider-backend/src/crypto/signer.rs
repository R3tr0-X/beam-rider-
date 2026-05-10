use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::error::AppError;

const DEFAULT_TEST_KEY: [u8; 32] = [42; 32];

#[derive(Clone)]
pub struct ResponseSigner {
    signing_key: SigningKey,
}

impl ResponseSigner {
    pub fn from_optional_secret(secret: Option<&str>) -> Result<Self, AppError> {
        let bytes = match secret {
            Some(value) => parse_secret(value)?,
            None => DEFAULT_TEST_KEY,
        };
        Ok(Self {
            signing_key: SigningKey::from_bytes(&bytes),
        })
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_bytes())
    }

    pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), AppError> {
        let key_bytes: [u8; 32] = public_key
            .try_into()
            .map_err(|_| AppError::Crypto("Ed25519 public key must be 32 bytes".to_string()))?;
        let signature_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| AppError::Crypto("Ed25519 signature must be 64 bytes".to_string()))?;
        let key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|err| AppError::Crypto(err.to_string()))?;
        let signature = Signature::from_bytes(&signature_bytes);
        key.verify(message, &signature)
            .map_err(|err| AppError::Crypto(err.to_string()))
    }
}

fn parse_secret(secret: &str) -> Result<[u8; 32], AppError> {
    let trimmed = secret.trim();
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);

    let decoded =
        if without_prefix.len() == 64 && without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
            hex::decode(without_prefix).map_err(|err| AppError::Crypto(err.to_string()))?
        } else {
            base64::engine::general_purpose::STANDARD
                .decode(trimmed)
                .map_err(|err| AppError::Crypto(err.to_string()))?
        };

    decoded
        .try_into()
        .map_err(|_| AppError::Crypto("ED25519_SIGNING_KEY must decode to 32 bytes".to_string()))
}
