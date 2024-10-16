//! Basic file encryption
use base64::Engine;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key as CryptoKey, KeyInit, Nonce};
use secrecy::zeroize::Zeroize;
use secrecy::{zeroize, SecretBox};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Base64: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Supplied data does not match expected size")]
    InvalidLength,
    #[error("No input was provided")]
    NoInput,
    #[error("Encryption Error")]
    Encryption,
    #[error("Decryption Error")]
    Decryption,
}

#[derive(Clone, Eq, PartialEq)]
/// Encryption Key for use with [`encrypt`] and [`decrypt`].
pub struct Key(CryptoKey);

impl Zeroize for Key {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl secrecy::CloneableSecret for Key {}

impl zeroize::ZeroizeOnDrop for Key {}

impl Drop for Key {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Key {
    /// Create a new random encryption key.
    #[must_use]
    pub fn new() -> SecretBox<Self> {
        let mut rng = OsRng {};
        SecretBox::new(Box::new(Self(ChaCha20Poly1305::generate_key(&mut rng))))
    }

    /// Create a new encryption key from a base64 encoded string.
    ///
    /// # Errors
    ///
    /// Returns error if the key is not valid or the string is not valid base64.
    pub fn with_base64(str: impl AsRef<str>) -> Result<SecretBox<Self>, Error> {
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::PAD,
        );

        let bytes = engine.decode(str.as_ref())?;
        let key = Self::try_from(bytes.as_slice())?;
        Ok(SecretBox::new(Box::new(key)))
    }

    /// Convert the current Key to a base64 string.
    #[must_use]
    pub fn to_base64(&self) -> String {
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::PAD,
        );
        engine.encode(self.0.as_slice())
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

const ENCRYPTION_KEY_BYTES_LEN: usize = 32;

impl From<[u8; ENCRYPTION_KEY_BYTES_LEN]> for Key {
    fn from(value: [u8; ENCRYPTION_KEY_BYTES_LEN]) -> Self {
        Self(CryptoKey::from(value))
    }
}

impl TryFrom<&[u8]> for Key {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != ENCRYPTION_KEY_BYTES_LEN {
            return Err(Error::InvalidLength);
        }

        let mut bytes = [0u8; ENCRYPTION_KEY_BYTES_LEN];
        bytes.copy_from_slice(value);
        Ok(bytes.into())
    }
}

impl Key {
    /// Encrypt the given `bytes`.
    ///
    /// # Errors
    ///
    /// Returns error if the encryption failed.
    pub fn encrypt(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        if bytes.is_empty() {
            return Err(Error::NoInput);
        }
        let mut rng = OsRng {};
        let nonce = ChaCha20Poly1305::generate_nonce(&mut rng);
        let cipher = ChaCha20Poly1305::new(&self.0);
        let mut encrypted = cipher
            .encrypt(&nonce, bytes)
            .map_err(|_| Error::Encryption)?;
        encrypted.extend_from_slice(nonce.as_slice());
        Ok(encrypted)
    }

    /// Decrypt the given `bytes`.
    ///
    /// # Errors
    ///
    /// Returns error if the decryption failed.
    pub fn decrypt(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        const NONCE_LEN: usize = 12;
        if bytes.len() < NONCE_LEN {
            return Err(Error::InvalidLength);
        }
        let data_len = bytes.len() - NONCE_LEN;
        let nonce = Nonce::from_slice(&bytes[data_len..]);
        debug_assert_eq!(nonce.len(), NONCE_LEN);
        let cipher = ChaCha20Poly1305::new(&self.0);
        let decrypted = cipher
            .decrypt(nonce, &bytes[0..data_len])
            .map_err(|_| Error::Decryption)?;
        Ok(decrypted)
    }
}

#[test]
fn test_encrypt_decrypt() {
    use secrecy::ExposeSecret;
    let value = b"Hello World!!";
    let key = Key::new();
    let encrypted = key.expose_secret().encrypt(value).unwrap();
    let decrypted = key.expose_secret().decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.as_slice(), value);
}
