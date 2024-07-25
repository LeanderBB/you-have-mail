//! Human Verification helpers for the Proton API.
//!
//! When you encounter an [`APIError`] check if [`APIError::is_human_verification_request()`]
//! returns true. If so retrieve the human verification data and perform the actions
//! specific to the type of human verification.
use serde::Deserialize;

/// Human Verification Type return by API.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
pub enum VerificationType {
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

impl VerificationType {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            VerificationType::Captcha => "captcha",
            VerificationType::Email => "email",
            VerificationType::Sms => "sms",
        }
    }
}

impl std::fmt::Display for VerificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Human Verification data required for Login.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginData {
    /// Type of human verification where the code originated from.
    pub hv_type: VerificationType,
    /// Result of the human verification request.
    pub token: String,
}

impl LoginData {
    /// Create HV login data from webview `data`.
    ///
    /// # Errors
    ///
    /// Returns error if the data does not match the expected format.
    pub fn from_webview_string(data: &str) -> serde_json::Result<Self> {
        serde_json::from_str(data)
    }
}

/// Information for the Human Verification request.
#[derive(Debug)]
pub struct HumanVerification {
    /// Types of supported verification.
    pub methods: Vec<VerificationType>,
    /// Token for the verification request.
    pub token: String,
}

/// When solving HTML Captcha, the webpage will post a JSON message, use this type to decode the
/// message.
#[derive(Debug, Deserialize)]
pub struct CaptchaMessage<'a> {
    #[serde(rename = "type")]
    message_type: &'a str,
    height: Option<usize>,
    token: Option<&'a str>,
}

impl<'a> CaptchaMessage<'a> {
    const CAPTCHA_MESSAGE_HEIGHT: &'static str = "pm_height";
    const CAPTCHA_MESSAGE_TOKEN: &'static str = "pm_captcha";
    const CAPTCHA_MESSAGE_EXPIRED: &'static str = "pm_captcha_expired";

    /// Deserialize the captcha message.
    ///
    /// # Errors
    /// Returns error if the deserialization failed.
    pub fn new(message: &'a str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<CaptchaMessage<'a>>(message)
    }

    /// Whether this message is a token message type.
    #[must_use]
    pub fn is_token(&self) -> bool {
        self.message_type == Self::CAPTCHA_MESSAGE_TOKEN && self.token.is_some()
    }

    /// Whether this message is a height message type.
    #[must_use]
    pub fn is_height(&self) -> bool {
        self.message_type == Self::CAPTCHA_MESSAGE_HEIGHT && self.height.is_some()
    }

    /// Whether the captcha has expired.
    #[must_use]
    pub fn is_captcha_expired(&self) -> bool {
        self.message_type == Self::CAPTCHA_MESSAGE_EXPIRED
    }

    /// Retrieve the token.
    ///
    /// Returns `None` if the message is not of type token.
    #[must_use]
    pub fn get_token(&self) -> Option<&str> {
        if !self.is_token() {
            return None;
        }

        self.token
    }

    /// Retrieve the height of the captcha.
    ///
    /// Returns `None` if the message is not of type height.
    #[must_use]
    pub fn get_height(&self) -> Option<usize> {
        if !self.is_height() {
            return None;
        }

        self.height
    }
}
