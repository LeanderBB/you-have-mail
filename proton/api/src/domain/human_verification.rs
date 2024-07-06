//! Human Verification helpers for the Proton API.
//!
//! When you encounter an [`APIError`] check if [`APIError::is_human_verification_request()`]
//! returns true. If so retrieve the human verification data and perform the actions
//! specific to the type of human verification.
use serde::Deserialize;

/// Human Verification Type return by API.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
pub enum HumanVerificationType {
    /// User needs to solve a Captcha, use [`crate::captcha_get`] to retrieve the token, solve in a web
    /// browser/view and retrieve the token posted via an `HVCaptchaMessage`.
    Captcha,
    /// User needs to verify via a token send via an email. Note: Request for this
    /// verification is not yet implemented.
    Email,
    /// User needs to verify via a token send via sms. Note: Request for this verification is not
    /// yet inmplemented.
    Sms,
}

impl HumanVerificationType {
    pub fn as_str(&self) -> &str {
        match self {
            HumanVerificationType::Captcha => "captcha",
            HumanVerificationType::Email => "email",
            HumanVerificationType::Sms => "sms",
        }
    }
}

impl std::fmt::Display for HumanVerificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Human Verification data required for Login.
#[derive(Debug, Clone)]
pub struct HumanVerificationLoginData {
    /// Type of human verification where the code originated from.
    pub hv_type: HumanVerificationType,
    /// Result of the human verification request.
    pub token: String,
}

/// Information for the Human Verification request.
#[derive(Debug)]
pub struct HumanVerification {
    /// Types of supported verification.
    pub methods: Vec<HumanVerificationType>,
    /// Token for the verification request.
    pub token: String,
}

/// When solving HTML Captcha, the webpage will post a JSON message, use this type to decode the
/// message.
#[derive(Debug, Deserialize)]
pub struct HVCaptchaMessage<'a> {
    #[serde(rename = "type")]
    message_type: &'a str,
    height: Option<usize>,
    token: Option<&'a str>,
}

impl<'a> HVCaptchaMessage<'a> {
    const CAPTCHA_MESSAGE_HEIGHT: &'static str = "pm_height";
    const CAPTCHA_MESSAGE_TOKEN: &'static str = "pm_captcha";
    const CAPTCHA_MESSAGE_EXPIRED: &'static str = "pm_captcha_expired";

    pub fn new(message: &'a str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<HVCaptchaMessage<'a>>(message)
    }

    pub fn is_token(&self) -> bool {
        self.message_type == Self::CAPTCHA_MESSAGE_TOKEN && self.token.is_some()
    }

    pub fn is_height(&self) -> bool {
        self.message_type == Self::CAPTCHA_MESSAGE_HEIGHT && self.height.is_some()
    }

    pub fn is_captcha_expired(&self) -> bool {
        self.message_type == Self::CAPTCHA_MESSAGE_EXPIRED
    }

    pub fn get_token(&self) -> Option<&str> {
        if !self.is_token() {
            return None;
        }

        self.token
    }

    pub fn get_height(&self) -> Option<usize> {
        if !self.is_height() {
            return None;
        }

        self.height
    }
}
