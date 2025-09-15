//! Domain Types.

pub mod errors;
pub mod event;
pub mod human_verification;
pub mod label;
pub mod message;
pub mod user;

use serde_repr::Deserialize_repr;
use std::fmt::{Display, Formatter};

pub type SecretString = secrecy::SecretString;
pub use secrecy::ExposeSecret;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
/// Types of Two Factor Authentication.
pub enum TwoFactorAuth {
    None,
    TOTP,
    FIDO2,
}

impl Display for TwoFactorAuth {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TwoFactorAuth::None => "None".fmt(f),
            TwoFactorAuth::TOTP => "TOTP".fmt(f),
            TwoFactorAuth::FIDO2 => "FIDO2".fmt(f),
        }
    }
}

#[derive(Debug, Deserialize_repr, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "mocks", derive(serde_repr::Serialize_repr))]
#[repr(u8)]
pub enum Boolean {
    False = 0,
    True = 1,
}

impl Default for Boolean {
    fn default() -> Self {
        Self::False
    }
}
