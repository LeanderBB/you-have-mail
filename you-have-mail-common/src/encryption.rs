//! Basic file encryption
use anyhow::anyhow;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key, KeyInit, Nonce};
use secrecy::{ExposeSecret, Secret};

/// Abstract the encryption of data.
pub trait Encryption {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, anyhow::Error>;
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, anyhow::Error>;
}

#[derive(Copy, Clone, Eq, PartialEq)]
/// Encryption Key for use with [`encrypt`] and [`decrypt`].
pub struct EncryptionKey(Key);

impl secrecy::Zeroize for EncryptionKey {
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl EncryptionKey {
    pub fn new() -> Secret<Self> {
        let mut rng = OsRng::default();
        Secret::new(Self(ChaCha20Poly1305::generate_key(&mut rng)))
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

/// Default encryption/decryption using chacha20poly1305.
pub struct DefaultEncryption {
    key: Secret<EncryptionKey>,
}

impl DefaultEncryption {
    pub fn new(key: Secret<EncryptionKey>) -> Self {
        Self { key }
    }
}

impl Encryption for DefaultEncryption {
    fn encrypt(&self, bytes: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        if bytes.is_empty() {
            return Err(anyhow!("Empty data"));
        }
        let mut rng = OsRng::default();
        let nonce = ChaCha20Poly1305::generate_nonce(&mut rng);
        let cipher = ChaCha20Poly1305::new(&self.key.expose_secret().0);
        let mut encrypted = cipher.encrypt(&nonce, bytes).map_err(|e| anyhow!(e))?;
        encrypted.extend_from_slice(nonce.as_slice());
        Ok(encrypted)
    }

    fn decrypt(&self, bytes: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        const NONCE_LEN: usize = 12;
        if bytes.len() < NONCE_LEN {
            return Err(anyhow!("Data is not large enough"));
        }
        let data_len = bytes.len() - NONCE_LEN;
        let nonce = Nonce::from_slice(&bytes[data_len..]);
        debug_assert_eq!(nonce.len(), NONCE_LEN);
        let cipher = ChaCha20Poly1305::new(&self.key.expose_secret().0);
        let decrypted = cipher
            .decrypt(nonce, &bytes[0..data_len])
            .map_err(|e| anyhow!(e))?;
        Ok(decrypted)
    }
}

#[test]
fn test_encrypt_decrypt() {
    let value = b"Hello World!!";
    let key = EncryptionKey::new();
    let encryptor = DefaultEncryption::new(key);
    let encrypted = encryptor.encrypt(value).unwrap();
    let decrypted = encryptor.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.as_slice(), value);
}
