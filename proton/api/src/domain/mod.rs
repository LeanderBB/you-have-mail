//! Domain Types.

mod event;
mod human_verification;
mod labels;
mod user;

pub use event::*;
pub use human_verification::*;
pub use labels::*;
pub use user::*;

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
