use crate::auth::{RefreshToken, Token, Uid};
use crate::domain::human_verification::LoginData;
use http::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use std::borrow::Cow;

#[doc(hidden)]
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PostAuthInfoRequest<'a> {
    pub username: &'a str,
}

impl<'a> http::Request for PostAuthInfoRequest<'a> {
    type Response = http::JsonResponse<PostAuthInfoResponse>;
    const METHOD: Method = Method::Post;

    fn url(&self) -> String {
        "auth/v4/info".to_owned()
    }

    fn build(&self, builder: RequestBuilder) -> http::Result<RequestBuilder> {
        Ok(builder.json(self))
    }
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PostAuthInfoResponse {
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
pub struct PostAuthRequest<'a> {
    pub username: &'a str,
    pub client_ephemeral: &'a str,
    pub client_proof: &'a str,
    #[serde(rename = "SRPSession")]
    pub srp_session: &'a str,
    #[serde(skip)]
    pub human_verification: Option<&'a LoginData>,
}

impl<'a> http::Request for PostAuthRequest<'a> {
    type Response = http::JsonResponse<PostAuthResponse>;
    const METHOD: Method = Method::Post;

    fn url(&self) -> String {
        "auth/v4".to_owned()
    }

    fn build(&self, mut builder: RequestBuilder) -> http::Result<RequestBuilder> {
        builder = builder.json(self);

        if let Some(hv) = &self.human_verification {
            // repeat submission with x-pm-human-verification-token and x-pm-human-verification-token-type
            builder = builder
                .header(X_PM_HUMAN_VERIFICATION_TOKEN, &hv.token)
                .header(X_PM_HUMAN_VERIFICATION_TOKEN_TYPE, hv.hv_type.as_str());
        }

        Ok(builder)
    }
}

#[doc(hidden)]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PostAuthResponse {
    #[serde(rename = "UserID")]
    pub user_id: String,
    #[serde(rename = "UID")]
    pub uid: Uid,
    pub token_type: Option<String>,
    pub access_token: Token,
    pub refresh_token: RefreshToken,
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
pub struct TFAAuthData<'a> {
    pub two_factor_code: &'a str,
    #[serde(rename = "FIDO2")]
    pub fido2: FIDO2AuthData<'a>,
}

#[doc(hidden)]
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FIDO2AuthData<'a> {
    pub authentication_options: serde_json::Value,
    pub client_data: &'a str,
    pub authentication_data: &'a str,
    pub signature: &'a str,
    #[serde(rename = "CredentialID")]
    pub credential_id: &'a [i32],
}

impl<'a> FIDO2AuthData<'a> {
    #[must_use]
    pub fn empty() -> Self {
        FIDO2AuthData {
            authentication_options: serde_json::Value::Null,
            client_data: "",
            authentication_data: "",
            signature: "",
            credential_id: &[],
        }
    }
}

pub struct PostTOTPRequest<'a> {
    code: &'a str,
}

impl<'a> PostTOTPRequest<'a> {
    #[must_use]
    pub fn new(code: &'a str) -> Self {
        Self { code }
    }
}

impl<'a> http::Request for PostTOTPRequest<'a> {
    type Response = http::NoResponse;
    const METHOD: Method = Method::Post;

    fn url(&self) -> String {
        "auth/v4/2fa".to_owned()
    }

    fn build(&self, builder: RequestBuilder) -> http::Result<RequestBuilder> {
        Ok(builder.json(TFAAuthData {
            two_factor_code: self.code,
            fido2: FIDO2AuthData::empty(),
        }))
    }
}

#[doc(hidden)]
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PostAuthRefresh<'a> {
    #[serde(rename = "UID")]
    pub uid: &'a str,
    pub refresh_token: &'a str,
    pub grant_type: &'a str,
    pub response_type: &'a str,
    #[serde(rename = "RedirectURI")]
    pub redirect_uri: &'a str,
}

#[doc(hidden)]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PostAuthRefreshResponse {
    #[serde(rename = "UID")]
    pub uid: Uid,
    pub token_type: Option<String>,
    pub access_token: Token,
    pub refresh_token: RefreshToken,
    pub scope: String,
}

pub struct PostAuthRefreshRequest<'a> {
    uid: &'a Uid,
    token: &'a str,
}

impl<'a> PostAuthRefreshRequest<'a> {
    #[must_use]
    pub fn new(uid: &'a Uid, token: &'a str) -> Self {
        Self { uid, token }
    }
}

impl<'a> http::Request for PostAuthRefreshRequest<'a> {
    type Response = http::JsonResponse<PostAuthRefreshResponse>;
    const METHOD: Method = Method::Post;

    fn url(&self) -> String {
        "auth/v4/refresh".to_owned()
    }

    fn build(&self, builder: RequestBuilder) -> http::Result<RequestBuilder> {
        Ok(builder.json(PostAuthRefresh {
            uid: &self.uid.0,
            refresh_token: self.token,
            grant_type: "refresh_token",
            response_type: "token",
            redirect_uri: "https://protonmail.ch/",
        }))
    }
}

#[derive(Copy, Clone)]
pub(crate) struct LogoutRequest {}

impl http::Request for LogoutRequest {
    type Response = http::NoResponse;
    const METHOD: Method = Method::Delete;

    fn url(&self) -> String {
        "auth/v4".to_owned()
    }

    fn build(&self, builder: RequestBuilder) -> http::Result<RequestBuilder> {
        Ok(builder)
    }
}

pub struct GetCaptchaRequest<'a> {
    token: &'a str,
    force_web: bool,
}

impl<'a> GetCaptchaRequest<'a> {
    #[must_use]
    pub fn new(token: &'a str, force_web: bool) -> Self {
        Self { token, force_web }
    }
}

impl<'a> http::Request for GetCaptchaRequest<'a> {
    type Response = http::StringResponse;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "core/v4/captcha".to_owned()
    }

    fn build(&self, mut builder: RequestBuilder) -> http::Result<RequestBuilder> {
        if self.force_web {
            builder = builder.query("ForceWebMessaging", "1");
        }

        Ok(builder.query("Token", self.token))
    }
}

const X_PM_HUMAN_VERIFICATION_TOKEN: &str = "X-Pm-Human-Verification-Token";
const X_PM_HUMAN_VERIFICATION_TOKEN_TYPE: &str = "X-Pm-Human-Verification-Token-Type";
