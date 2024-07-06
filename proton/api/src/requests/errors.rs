use crate::domain::{HumanVerification, HumanVerificationType};
use anyhow::anyhow;
use serde::Deserialize;
use thiserror::Error;

const HUMAN_VERIFICATION_REQUESTED: u32 = 9001;

#[derive(Deserialize)]
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
    pub fn is_human_verification_request(&self) -> bool {
        self.api_code == HUMAN_VERIFICATION_REQUESTED
    }

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
                "captcha" => HumanVerificationType::Captcha,
                "email" => HumanVerificationType::Email,
                "sms" => HumanVerificationType::Email,
                _ => {
                    return Err(GetHumanVerificationError::UnknownVerificationType(
                        t.clone(),
                    ))
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
            write!(f, "APIError code={} http={}", self.api_code, self.http_code)
        }
    }
}

impl APIError {
    pub fn new(http_status: u16) -> Self {
        Self {
            http_code: http_status,
            api_code: 0,
            message: None,
            details: None,
        }
    }

    pub fn with_status_and_body(http_status: u16, body: &[u8]) -> Self {
        if body.is_empty() {
            return Self::new(http_status);
        }

        match serde_json::from_slice::<APIErrorDesc>(body) {
            Ok(e) => Self {
                http_code: http_status,
                api_code: e.code,
                message: e.error,
                details: e.details,
            },
            Err(_) => Self::new(http_status),
        }
    }
}
