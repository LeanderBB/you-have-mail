use crate::yhm::YhmError;
use parking_lot::Mutex;
use std::fmt::{Display, Formatter};
use yhm::backend::proton::proton_api;
use you_have_mail_common as yhm;
use you_have_mail_common::backend::proton::proton_api::domain::human_verification::{
    HumanVerification, LoginData, VerificationType,
};
use you_have_mail_common::backend::proton::proton_api::requests::GetCaptchaRequest;
use you_have_mail_common::yhm::IntoAccount;

#[derive(Debug, uniffi::Enum)]
pub enum ProtonHumanVerificationType {
    Captcha,
    Email,
    Sms,
}

impl Display for ProtonHumanVerificationType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtonHumanVerificationType::Captcha => {
                write!(f, "Captcha")
            }
            ProtonHumanVerificationType::Email => {
                write!(f, "Email")
            }
            ProtonHumanVerificationType::Sms => {
                write!(f, "Sms")
            }
        }
    }
}

impl From<VerificationType> for ProtonHumanVerificationType {
    fn from(value: VerificationType) -> Self {
        match value {
            VerificationType::Captcha => Self::Captcha,
            VerificationType::Email => Self::Email,
            VerificationType::Sms => Self::Sms,
        }
    }
}

impl From<ProtonHumanVerificationType> for VerificationType {
    fn from(value: ProtonHumanVerificationType) -> Self {
        match value {
            ProtonHumanVerificationType::Captcha => Self::Captcha,
            ProtonHumanVerificationType::Email => Self::Sms,
            ProtonHumanVerificationType::Sms => Self::Email,
        }
    }
}

#[derive(Debug, uniffi::Record)]
pub struct ProtonHumanVerification {
    /// Types of supported verification.
    pub methods: Vec<ProtonHumanVerificationType>,
    /// Token for the verification request.
    pub token: String,
}

impl From<HumanVerification> for ProtonHumanVerification {
    fn from(value: HumanVerification) -> Self {
        Self {
            methods: value
                .methods
                .into_iter()
                .map(ProtonHumanVerificationType::from)
                .collect(),
            token: value.token,
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ProtonLoginError {
    #[error("API: {0}")]
    Api(String),
    #[error("Http: {0}")]
    Http(String),
    #[error("Can not perform operation in the current state")]
    InvalidState,
    #[error("Server SRP proof verification failed: {0}")]
    SRPServerProof(String),
    #[error("SRP: {0}")]
    SRP(String),
    #[error("Account 2FA method ({0}) is not supported")]
    Unsupported2FA(String),
    #[error("Invalid Human Verification Data ")]
    InvalidHumanVerificationData,
    #[error("Human Verification Required'")]
    HumanVerificationRequired(ProtonHumanVerification),
    #[error("Unsupported Human Verification:{0}")]
    HumanVerificationTypeNotSupported(ProtonHumanVerificationType),
    #[error("Auth Store:{0}")]
    AuthStore(String),
    #[error("The object has become invalid")]
    Invalid,
    #[error("Create Account: {0}")]
    CreateAccount(#[from] YhmError),
}

impl From<proton_api::login::Error> for ProtonLoginError {
    fn from(value: proton_api::login::Error) -> Self {
        use proton_api::login::Error;
        match value {
            Error::Api(e) => Self::Api(e.to_string()),
            Error::Http(e) => Self::Http(e.to_string()),
            Error::InvalidState => Self::InvalidState,
            Error::SRPServerProof(e) => Self::SRPServerProof(e),
            Error::SRP(e) => Self::SRP(e.to_string()),
            Error::Unsupported2FA(e) => Self::Unsupported2FA(e.to_string()),
            Error::HumanVerificationRequired(hv) => Self::HumanVerificationRequired(hv.into()),
            Error::HumanVerificationTypeNotSupported(hv) => {
                Self::HumanVerificationTypeNotSupported(hv.into())
            }
            Error::AuthStore(e) => Self::AuthStore(e.to_string()),
        }
    }
}

impl From<yhm::you_have_mail_http::Error> for ProtonLoginError {
    fn from(value: yhm::you_have_mail_http::Error) -> Self {
        Self::Http(value.to_string())
    }
}

/// Guides the user through the process of integrating with a Proton account.
#[derive(uniffi::Object)]

pub struct ProtonLoginSequence {
    sequence: Mutex<Option<proton_api::login::Sequence>>,
}

#[uniffi::export]
impl ProtonLoginSequence {
    /// Create new instance.
    ///
    /// # Errors
    ///
    /// Returns error if the client fails to build.
    #[uniffi::constructor]
    pub fn new(proxy: Option<crate::proxy::Proxy>) -> Result<Self, ProtonLoginError> {
        let sequence = yhm::backend::proton::Backend::login_sequence(proxy.map(Into::into))?;

        Ok(Self {
            sequence: Mutex::new(Some(sequence)),
        })
    }

    /// Check whether the account is logged in.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn is_logged_in(&self) -> Result<bool, ProtonLoginError> {
        let guard = self.sequence.lock();
        let Some(sequence) = &*guard else {
            return Err(ProtonLoginError::Invalid);
        };

        Ok(sequence.is_logged_in())
    }

    /// Check whether the account is logged out.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn is_logged_out(&self) -> Result<bool, ProtonLoginError> {
        let guard = self.sequence.lock();
        let Some(sequence) = &*guard else {
            return Err(ProtonLoginError::Invalid);
        };

        Ok(sequence.is_logged_out())
    }

    /// Check whether the account is awaiting two factor authentication.
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn is_awaiting_totp(&self) -> Result<bool, ProtonLoginError> {
        let guard = self.sequence.lock();
        let Some(sequence) = &*guard else {
            return Err(ProtonLoginError::Invalid);
        };

        Ok(sequence.is_awaiting_totp())
    }

    /// Login into account with `email` and `password`.
    ///
    /// Human verification data can be passed in with `human_verification`.
    ///
    /// # Errors
    ///
    /// Returns error if the request failed.
    pub fn login(
        &self,
        email: &str,
        password: &str,
        human_verification: Option<String>,
    ) -> Result<(), ProtonLoginError> {
        let login_data = if let Some(hv_data) = human_verification {
            Some(LoginData::from_webview_string(&hv_data).map_err(|e| {
                tracing::error!("Failed to deserialize hv data: {e}");
                ProtonLoginError::InvalidHumanVerificationData
            })?)
        } else {
            None
        };

        let mut guard = self.sequence.lock();
        let Some(sequence) = &mut *guard else {
            return Err(ProtonLoginError::Invalid);
        };

        Ok(sequence.login(email, password, login_data.as_ref())?)
    }

    /// Submit `totp` code.
    ///
    /// # Errors
    ///
    /// Returns error if the request failed.
    pub fn submit_totp(&self, code: &str) -> Result<(), ProtonLoginError> {
        let mut guard = self.sequence.lock();
        let Some(sequence) = &mut *guard else {
            return Err(ProtonLoginError::Invalid);
        };

        Ok(sequence.submit_totp(code)?)
    }

    /// Convert into a usable account.
    ///
    /// Note that after this operation the object becomes invalid, whether we succeed
    /// or fail.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn create_account(&self, yhm: &crate::yhm::Yhm) -> Result<(), ProtonLoginError> {
        let mut guard = self.sequence.lock();
        let Some(sequence) = guard.take() else {
            return Err(ProtonLoginError::Invalid);
        };

        Ok(sequence
            .into_account(yhm.instance())
            .map_err(YhmError::from)?)
    }

    /// Get the captcha html code for a given token.
    ///
    /// # Errors
    ///
    /// Returns error if request failed.
    pub fn captcha(&self, token: &str) -> Result<String, ProtonLoginError> {
        let mut guard = self.sequence.lock();
        let Some(sequence) = guard.take() else {
            return Err(ProtonLoginError::Invalid);
        };

        Ok(sequence
            .session()
            .execute(GetCaptchaRequest::new(token, false))?)
    }
}
