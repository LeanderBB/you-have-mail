use crate::domain::{HumanVerificationLoginData, SecretString, UserUid};
use crate::http;
use crate::http::{RequestData, X_PM_HUMAN_VERIFICATION_TOKEN, X_PM_HUMAN_VERIFICATION_TOKEN_TYPE};
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use std::borrow::Cow;

#[doc(hidden)]
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthInfoRequest<'a> {
    pub username: &'a str,
}

impl<'a> http::RequestDesc for AuthInfoRequest<'a> {
    type Output = AuthInfoResponse;
    type Response = http::JsonResponse<Self::Output>;

    fn build(&self) -> RequestData {
        RequestData::new(http::Method::Post, "auth/v4/info").json(self)
    }
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AuthInfoResponse {
    pub version: i64,
    pub modulus: String,
    pub server_ephemeral: String,
    pub salt: String,
    #[serde(rename = "SRPSession")]
    pub srp_session: String,
}

#[doc(hidden)]
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthRequest<'a> {
    pub username: &'a str,
    pub client_ephemeral: &'a str,
    pub client_proof: &'a str,
    #[serde(rename = "SRPSession")]
    pub srp_session: &'a str,
    #[serde(skip)]
    pub human_verification: &'a Option<HumanVerificationLoginData>,
}

impl<'a> http::RequestDesc for AuthRequest<'a> {
    type Output = AuthResponse;
    type Response = http::JsonResponse<Self::Output>;

    fn build(&self) -> RequestData {
        let mut request = RequestData::new(http::Method::Post, "auth/v4").json(self);

        if let Some(hv) = &self.human_verification {
            // repeat submission with x-pm-human-verification-token and x-pm-human-verification-token-type
            request = request
                .header(X_PM_HUMAN_VERIFICATION_TOKEN, &hv.token)
                .header(X_PM_HUMAN_VERIFICATION_TOKEN_TYPE, hv.hv_type.as_str())
        }

        request
    }
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AuthResponse {
    #[serde(rename = "UserID")]
    pub user_id: String,
    #[serde(rename = "UID")]
    pub uid: String,
    pub token_type: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub server_proof: String,
    pub scope: String,
    #[serde(rename = "2FA")]
    pub tfa: TFAInfo,
    pub password_mode: PasswordMode,
}

#[doc(hidden)]
#[derive(Deserialize_repr, Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum PasswordMode {
    One = 1,
    Two = 2,
}

#[doc(hidden)]
#[derive(Deserialize_repr, Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum TFAStatus {
    None = 0,
    Totp = 1,
    FIDO2 = 2,
    TotpOrFIDO2 = 3,
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct TFAInfo {
    pub enabled: TFAStatus,
    #[serde(rename = "FIDO2")]
    pub fido2_info: FIDO2Info,
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FIDOKey<'a> {
    pub attestation_format: Cow<'a, str>,
    #[serde(rename = "CredentialID")]
    pub credential_id: Vec<i32>,
    pub name: Cow<'a, str>,
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FIDO2Info {
    pub authentication_options: serde_json::Value,
    pub registered_keys: Option<serde_json::Value>,
}

#[doc(hidden)]
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct TFAAuth<'a> {
    pub two_factor_code: &'a str,
    #[serde(rename = "FIDO2")]
    pub fido2: FIDO2Auth<'a>,
}

#[doc(hidden)]
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FIDO2Auth<'a> {
    pub authentication_options: serde_json::Value,
    pub client_data: &'a str,
    pub authentication_data: &'a str,
    pub signature: &'a str,
    #[serde(rename = "CredentialID")]
    pub credential_id: &'a [i32],
}

impl<'a> FIDO2Auth<'a> {
    pub fn empty() -> Self {
        FIDO2Auth {
            authentication_options: serde_json::Value::Null,
            client_data: "",
            authentication_data: "",
            signature: "",
            credential_id: &[],
        }
    }
}

pub struct TOTPRequest<'a> {
    code: &'a str,
}

impl<'a> TOTPRequest<'a> {
    pub fn new(code: &'a str) -> Self {
        Self { code }
    }
}

impl<'a> http::RequestDesc for TOTPRequest<'a> {
    type Output = ();
    type Response = http::NoResponse;

    fn build(&self) -> RequestData {
        RequestData::new(http::Method::Post, "auth/v4/2fa").json(TFAAuth {
            two_factor_code: self.code,
            fido2: FIDO2Auth::empty(),
        })
    }
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct UserAuth {
    pub uid: Secret<UserUid>,
    pub access_token: SecretString,
    pub refresh_token: SecretString,
}

impl UserAuth {
    pub fn from_auth_response(auth: AuthResponse) -> Self {
        Self {
            uid: Secret::new(UserUid(auth.uid)),
            access_token: SecretString::new(auth.access_token),
            refresh_token: SecretString::new(auth.refresh_token),
        }
    }

    pub fn from_auth_refresh_response(auth: AuthRefreshResponse) -> Self {
        Self {
            uid: Secret::new(UserUid(auth.uid)),
            access_token: SecretString::new(auth.access_token),
            refresh_token: SecretString::new(auth.refresh_token),
        }
    }
}

#[doc(hidden)]
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AuthRefresh<'a> {
    #[serde(rename = "UID")]
    pub uid: &'a str,
    pub refresh_token: &'a str,
    pub grant_type: &'a str,
    pub response_type: &'a str,
    #[serde(rename = "RedirectURI")]
    pub redirect_uri: &'a str,
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AuthRefreshResponse {
    #[serde(rename = "UID")]
    pub uid: String,
    pub token_type: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub scope: String,
}

pub struct AuthRefreshRequest<'a> {
    uid: &'a UserUid,
    token: &'a str,
}

impl<'a> AuthRefreshRequest<'a> {
    pub fn new(uid: &'a UserUid, token: &'a str) -> Self {
        Self { uid, token }
    }
}

impl<'a> http::RequestDesc for AuthRefreshRequest<'a> {
    type Output = AuthRefreshResponse;
    type Response = http::JsonResponse<Self::Output>;

    fn build(&self) -> RequestData {
        RequestData::new(http::Method::Post, "auth/v4/refresh").json(AuthRefresh {
            uid: &self.uid.0,
            refresh_token: self.token,
            grant_type: "refresh_token",
            response_type: "token",
            redirect_uri: "https://protonmail.ch/",
        })
    }
}

pub struct LogoutRequest {}

impl http::RequestDesc for LogoutRequest {
    type Output = ();
    type Response = http::NoResponse;

    fn build(&self) -> RequestData {
        RequestData::new(http::Method::Delete, "auth/v4")
    }
}

pub struct CaptchaRequest<'a> {
    token: &'a str,
    force_web: bool,
}

impl<'a> CaptchaRequest<'a> {
    pub fn new(token: &'a str, force_web: bool) -> Self {
        Self { token, force_web }
    }
}

impl<'a> http::RequestDesc for CaptchaRequest<'a> {
    type Output = String;
    type Response = http::StringResponse;

    fn build(&self) -> RequestData {
        let url = if self.force_web {
            format!("core/v4/captcha?ForceWebMessaging=1&Token={}", self.token)
        } else {
            format!("core/v4/captcha?Token={}", self.token)
        };

        RequestData::new(http::Method::Get, url)
    }
}
