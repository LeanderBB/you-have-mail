//! Basic file encryption
use anyhow::anyhow;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key, KeyInit, Nonce};
use secrecy::Secret;

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

impl From<[u8; 32]> for EncryptionKey {
    fn from(value: [u8; 32]) -> Self {
        Self(Key::from(value))
    }
}

pub fn encrypt(key: &EncryptionKey, bytes: &[u8]) -> Result<Box<[u8]>, anyhow::Error> {
    if bytes.is_empty() {
        return Err(anyhow!("Empty data"));
    }
    let mut rng = OsRng::default();
    let nonce = ChaCha20Poly1305::generate_nonce(&mut rng);
    let cipher = ChaCha20Poly1305::new(&key.0);
    let mut encrypted = cipher.encrypt(&nonce, bytes).map_err(|e| anyhow!(e))?;
    encrypted.extend_from_slice(nonce.as_slice());
    Ok(encrypted.into_boxed_slice())
}

pub fn decrypt(key: &EncryptionKey, bytes: &[u8]) -> Result<Box<[u8]>, anyhow::Error> {
    const NONCE_LEN: usize = 12;
    if bytes.len() < NONCE_LEN {
        return Err(anyhow!("Data is not large enough"));
    }
    let data_len = bytes.len() - NONCE_LEN;
    let nonce = Nonce::from_slice(&bytes[data_len..]);
    debug_assert_eq!(nonce.len(), NONCE_LEN);
    let cipher = ChaCha20Poly1305::new(&key.0);
    let decrypted = cipher
        .decrypt(nonce, &bytes[0..data_len])
        .map_err(|e| anyhow!(e))?;
    Ok(decrypted.into_boxed_slice())
}

#[test]
fn test_encrypt_decrypt() {
    use secrecy::ExposeSecret;

    let value = b"Hello World!!";
    let key = EncryptionKey::new();
    let encrypted = encrypt(key.expose_secret(), value).unwrap();
    let decrypted = decrypt(key.expose_secret(), &encrypted).unwrap();
    assert_eq!(decrypted.as_ref(), value);
}
