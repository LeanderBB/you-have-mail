use crate::domain::human_verification::{HumanVerification, VerificationType};
use anyhow::anyhow;
use serde::Deserialize;
use thiserror::Error;
use tracing::error;
use you_have_mail_http::http::Response;
use you_have_mail_http::{ExtSafeResponse, ureq};

pub const OPERATION_SUCCESS: u32 = 1000;
const HUMAN_VERIFICATION_REQUESTED: u32 = 9001;

/// Error status and details returned from proton server.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize))]
#[serde(rename_all = "PascalCase")]
pub struct APIErrorDesc {
    pub code: u32,
    pub error: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// Representation of the Proton API Error.
#[derive(Debug, Error)]
pub struct APIError {
    /// Http Code for the error.
    pub http_code: u16,
    /// Internal API code. Unfortunately, there is no public documentation for these values.
    pub api_code: u32,
    /// Optional error message that may be present.
    pub message: Option<String>,
    /// Optional JSON type with error details.
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum GetHumanVerificationError {
    #[error("This error is not a human verification request")]
    NotHumanVerificationError,
    #[error("Failed to deserialize human verification data:{0}")]
    Deserialize(#[source] anyhow::Error),
    #[error("Unknown human verification type '{0}'")]
    UnknownVerificationType(String),
}

impl APIError {
    /// Check whether this error is a request to perform HV.
    #[must_use]
    pub fn is_human_verification_request(&self) -> bool {
        self.api_code == HUMAN_VERIFICATION_REQUESTED
    }

    /// Attempt to decode the HV verification details.
    ///
    /// # Errors
    /// Returns error if we failed to extract the HV details.
    pub fn try_get_human_verification_details(
        &self,
    ) -> Result<HumanVerification, GetHumanVerificationError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub struct HumanVerificationData {
            pub human_verification_methods: Vec<String>,
            pub human_verification_token: String,
        }

        if !self.is_human_verification_request() {
            return Err(GetHumanVerificationError::NotHumanVerificationError);
        }

        let Some(details) = &self.details else {
            return Err(GetHumanVerificationError::Deserialize(anyhow!(
                "Error details are missing"
            )));
        };

        let hv = serde_json::from_value::<HumanVerificationData>(details.clone())
            .map_err(|e| GetHumanVerificationError::Deserialize(e.into()))?;

        let mut hv_types = Vec::with_capacity(hv.human_verification_methods.len());

        for t in &hv.human_verification_methods {
            let hv_type = match t.as_ref() {
                "captcha" => VerificationType::Captcha,
                "email" => VerificationType::Email,
                "sms" => VerificationType::Sms,
                _ => {
                    return Err(GetHumanVerificationError::UnknownVerificationType(
                        t.clone(),
                    ));
                }
            };
            hv_types.push(hv_type);
        }

        Ok(HumanVerification {
            token: hv.human_verification_token,
            methods: hv_types,
        })
    }
}

impl std::fmt::Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(m) = &self.message {
            m.fmt(f)
        } else {
            write!(
                f,
                "APIError code={} you-have-mail-http={}",
                self.api_code, self.http_code
            )
        }
    }
}

impl APIError {
    /// Create a new instance based of status code and response body.
    ///
    /// Note that if we fail to parse the response json only the you-have-mail-http status code is returned.
    pub fn with_status_and_response(status: u16, mut response: Response<ureq::Body>) -> Self {
        match serde_json::from_reader::<_, APIErrorDesc>(response.safe_reader()) {
            Ok(desc) => Self {
                http_code: status,
                api_code: desc.code,
                message: desc.error,
                details: desc.details,
            },
            Err(e) => {
                error!("Failed to decode API error string: {e}");
                Self::new(status)
            }
        }
    }

    /// Create a new instance with an `http_status` code.
    #[must_use]
    pub fn new(http_status: u16) -> Self {
        Self {
            http_code: http_status,
            api_code: 0,
            message: None,
            details: None,
        }
    }

    /// Create a new instance with an `http_status` code and `description`.
    #[must_use]
    pub fn with_desc(http_status: u16, description: APIErrorDesc) -> Self {
        Self {
            http_code: http_status,
            api_code: description.code,
            message: description.error,
            details: description.details,
        }
    }
}
