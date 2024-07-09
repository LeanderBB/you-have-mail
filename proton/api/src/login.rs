use crate::auth::{Auth, StoreError};
use crate::domain::errors::APIError;
use crate::domain::human_verification::{HumanVerification, LoginData};
use crate::domain::TwoFactorAuth;
use crate::requests::{PostAuthInfoRequest, PostAuthRequest, PostTOTPRequest, TFAStatus};
use crate::session::Session;
use go_srp::SRPAuth;
use std::sync::Arc;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API: {0}")]
    Api(APIError),
    #[error("Http: {0}")]
    Http(http::Error),
    #[error("Can not perform operation in the current state")]
    InvalidState,
    #[error("Server SRP proof verification failed: {0}")]
    SRPServerProof(String),
    #[error("Failed to calculate SRP Proof: {0}")]
    SRPProof(String),
    #[error("Account 2FA method ({0}) is not supported")]
    Unsupported2FA(TwoFactorAuth),
    #[error("Human Verification Required'")]
    HumanVerificationRequired(HumanVerification),
    #[error("Auth Store:{0}")]
    AuthStore(#[from] StoreError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<http::Error> for Error {
    fn from(value: http::Error) -> Self {
        match value {
            http::Error::Http(code, response) => {
                let api_err = APIError::with_status_and_response(code, response);
                if let Ok(hv) = api_err.try_get_human_verification_details() {
                    Self::HumanVerificationRequired(hv)
                } else {
                    Self::Api(api_err)
                }
            }
            _ => Self::Http(value),
        }
    }
}

/// Guides the user through the login sequence for a proton account.
///
/// The accounts start of with the usual email and password exchange.
///
/// If enabled, the next step is 2FA.
pub struct Sequence {
    session: Arc<Session>,
    state: State,
}

impl Sequence {
    /// Create a new instance with a given `session`.
    #[must_use]
    pub fn new(session: Arc<Session>) -> Self {
        Self {
            session,
            state: State::LoggedOut,
        }
    }

    /// Whether the account is waiting on totp code.
    #[must_use]
    pub fn is_awaiting_totp(&self) -> bool {
        matches!(self.state, State::AwaitingTotp)
    }

    /// Whether the account is logged out.
    #[must_use]
    pub fn is_logged_out(&self) -> bool {
        matches!(self.state, State::LoggedOut)
    }

    /// Check whether the login process has completed.
    #[must_use]
    pub fn is_logged_in(&self) -> bool {
        matches!(self.state, State::LoggedIn)
    }

    /// Login with `email` and `password`.
    ///
    /// If [`Error::HumanVerificationRequired`] is returned, you need to resolve the challenge
    /// and retry again with the resulting value for `human_verification_login_data`.
    ///
    /// # Errors
    /// Returns error if the request or the auth store failed, 2FA method is not supported
    /// or HV validation was requested.
    ///
    /// [`Error::InvalidState`] is returned if the sequence is not in a logged out state.
    pub fn login(
        &mut self,
        email: &str,
        password: &str,
        human_verification_login_data: Option<&LoginData>,
    ) -> Result<()> {
        if !matches!(self.state, State::LoggedOut) {
            return Err(Error::InvalidState);
        };

        let auth_info_response = self
            .session
            .execute(PostAuthInfoRequest { username: email })
            .map_err(|e| {
                error!("Failed to get auth info: {e}");
                e
            })?;

        let srp_auth = SRPAuth::generate(
            email,
            password,
            auth_info_response.version,
            &auth_info_response.salt,
            &auth_info_response.modulus,
            &auth_info_response.server_ephemeral,
        )
        .map_err(Error::SRPServerProof)?;

        let auth_response = self
            .session
            .execute(PostAuthRequest {
                username: email,
                client_ephemeral: &srp_auth.client_ephemeral,
                client_proof: &srp_auth.client_proof,
                srp_session: &auth_info_response.srp_session,
                human_verification: human_verification_login_data,
            })
            .map_err(|e| {
                error!("Failed to get auth response: {e}");
                e
            })?;

        if srp_auth.expected_server_proof != auth_response.server_proof {
            return Err(Error::SRPServerProof(
                "Server Proof does not match".to_string(),
            ));
        }

        match auth_response.tfa.enabled {
            TFAStatus::None => {
                self.state = State::LoggedIn;
            }
            TFAStatus::Totp | TFAStatus::TotpOrFIDO2 => {
                self.state = State::AwaitingTotp;
            }
            TFAStatus::FIDO2 => return Err(Error::Unsupported2FA(TwoFactorAuth::FIDO2)),
        }

        let mut guard = self.session.auth_store().write();
        guard
            .store(Auth {
                uid: auth_response.uid,
                auth_token: auth_response.access_token,
                refresh_token: auth_response.refresh_token,
            })
            .map_err(|e| {
                error!("Failed to write authentication data to store: {e}");
                self.state = State::LoggedOut;
                e
            })?;

        Ok(())
    }

    /// Submit `totp` 2FA Code
    ///
    /// To check if the sequence needs a totp 2fa user [`is_awaiting_totp()`].
    ///
    /// # Errors
    /// Returns error if the request failed.
    ///
    /// [`Error::InvalidState`] is returned if the sequence is not in a logged out state.
    pub fn submit_totp(&mut self, totp: &str) -> Result<()> {
        if !matches!(self.state, State::AwaitingTotp) {
            return Err(Error::InvalidState);
        };
        self.session
            .execute_with_auth(PostTOTPRequest::new(totp))
            .map_err(|e| {
                error!("Failed to submit totp code: {e}");
                e
            })?;

        self.state = State::LoggedIn;
        Ok(())
    }

    /// Abort login by triggering a logout
    ///
    /// # Errors
    /// Returns error if we are not in a valid state or the request failed.
    pub fn logout(&mut self) -> Result<()> {
        if !matches!(self.state, State::AwaitingTotp) {
            return Err(Error::InvalidState);
        };

        Ok(self.session.logout()?)
    }

    /// Conclude the login process.
    ///
    /// # Errors
    /// If the state is not logged in, the sequence will be returned as error.
    pub fn finish(self) -> std::result::Result<Arc<Session>, Self> {
        if !matches!(self.state, State::LoggedIn) {
            return Err(self);
        }

        Ok(self.session)
    }

    /// Get the underlying session.
    #[must_use]
    pub fn session(&self) -> &Session {
        self.session.as_ref()
    }
}

enum State {
    LoggedOut,
    AwaitingTotp,
    LoggedIn,
}
