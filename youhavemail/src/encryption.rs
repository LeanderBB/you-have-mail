//! Basic file encryption
use anyhow::anyhow;
use base64::Engine;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key, KeyInit, Nonce};
use secrecy::{ExposeSecret, Secret};

#[derive(Copy, Clone, Eq, PartialEq)]
/// Encryption Key for use with [`encrypt`] and [`decrypt`].
pub struct EncryptionKey(Key);

impl secrecy::Zeroize for EncryptionKey {
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl secrecy::CloneableSecret for EncryptionKey {}

impl EncryptionKey {
    pub fn new() -> Secret<Self> {
        let mut rng = OsRng {};
        Secret::new(Self(ChaCha20Poly1305::generate_key(&mut rng)))
    }

    pub fn with_base64(str: impl AsRef<str>) -> Result<Self, anyhow::Error> {
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::PAD,
        );

        let bytes = engine
            .decode(str.as_ref())
            .map_err(|e| anyhow!("Failed to decode base64: {e}"))?;
        let key = Self::try_from(bytes.as_slice()).map_err(|_| anyhow!("Invalid Key bytes"))?;
        Ok(key)
    }

    pub fn to_base64(&self) -> String {
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::PAD,
        );
        engine.encode(self.0.as_slice())
    }
}

impl AsRef<[u8]> for EncryptionKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

const ENCRYPTION_KEY_BYTES_LEN: usize = 32;

impl From<[u8; ENCRYPTION_KEY_BYTES_LEN]> for EncryptionKey {
    fn from(value: [u8; ENCRYPTION_KEY_BYTES_LEN]) -> Self {
        Self(Key::from(value))
    }
}

impl TryFrom<&[u8]> for EncryptionKey {
    type Error = ();
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != ENCRYPTION_KEY_BYTES_LEN {
            return Err(());
        }

        let mut bytes = [0u8; ENCRYPTION_KEY_BYTES_LEN];
        bytes.copy_from_slice(value);
        Ok(bytes.into())
    }
}

pub fn encrypt(key: &Secret<EncryptionKey>, bytes: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    if bytes.is_empty() {
        return Err(anyhow!("Empty data"));
    }
    let mut rng = OsRng {};
    let nonce = ChaCha20Poly1305::generate_nonce(&mut rng);
    let cipher = ChaCha20Poly1305::new(&key.expose_secret().0);
    let mut encrypted = cipher.encrypt(&nonce, bytes).map_err(|e| anyhow!(e))?;
    encrypted.extend_from_slice(nonce.as_slice());
    Ok(encrypted)
}

pub fn decrypt(key: &Secret<EncryptionKey>, bytes: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    const NONCE_LEN: usize = 12;
    if bytes.len() < NONCE_LEN {
        return Err(anyhow!("Data is not large enough"));
    }
    let data_len = bytes.len() - NONCE_LEN;
    let nonce = Nonce::from_slice(&bytes[data_len..]);
    debug_assert_eq!(nonce.len(), NONCE_LEN);
    let cipher = ChaCha20Poly1305::new(&key.expose_secret().0);
    let decrypted = cipher
        .decrypt(nonce, &bytes[0..data_len])
        .map_err(|e| anyhow!(e))?;
    Ok(decrypted)
}

#[test]
fn test_encrypt_decrypt() {
    let value = b"Hello World!!";
    let key = EncryptionKey::new();
    let encrypted = encrypt(&key, value).unwrap();
    let decrypted = decrypt(&key, &encrypted).unwrap();
    assert_eq!(decrypted.as_slice(), value);
}
