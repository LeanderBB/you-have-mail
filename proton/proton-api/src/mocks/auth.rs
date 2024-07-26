//! This module contains a couple of authentication responses captured with go-proton-api library
//! in order to mock the login sequence which requires crypto graphic checks.
//!

use crate::session::{DEFAULT_APP_VERSION, X_PM_APP_VERSION_HEADER, X_PM_UID_HEADER};
use mockito::{Mock, Server};

pub trait MatchExtension {
    /// Match against app_version version.
    fn match_version(self) -> Self;

    /// Match against authentication tokens.
    fn match_auth(self) -> Self;

    /// Match against authentication after refresh.
    fn match_auth_refreshed(self) -> Self;
}

impl MatchExtension for Mock {
    fn match_version(self) -> Self {
        self.match_header(X_PM_APP_VERSION_HEADER, DEFAULT_APP_VERSION)
    }

    fn match_auth(self) -> Self {
        self.match_header(X_PM_UID_HEADER, SESSION_UID)
            .match_header(AUTHORIZATION_HEADER_KEY, AUTHORIZATION_HEADER_VALUE)
            .match_version()
    }

    fn match_auth_refreshed(self) -> Self {
        self.match_header(X_PM_UID_HEADER, SESSION_UID)
            .match_header(
                AUTHORIZATION_HEADER_KEY,
                AUTHORIZATION_HEADER_POST_REFRESH_VALUE,
            )
            .match_version()
    }
}

/// Mock aut info state.
pub fn auth_info(server: &mut Server) -> Mock {
    server
        .mock("POST", "/auth/v4/info")
        .match_version()
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(AUTH_INFO_RESPONSE)
        .create()
}

/// Mock auth response.
///
/// Set `tfa` to true to trigger two factor authentication.
pub fn auth_response(server: &mut Server, tfa: bool) -> Mock {
    let mock = server
        .mock("POST", "/auth/v4")
        .match_version()
        .with_status(200);
    if tfa {
        mock.with_body(AUTH_RESPONSE_TFA)
    } else {
        mock.with_body(AUTH_RESPONSE)
    }
    .create()
}

/// Mock logout request.
pub fn logout(server: &mut Server) -> Mock {
    server
        .mock("DELETE", "/auth/v4")
        .match_version()
        .with_status(200)
        .create()
}

pub fn auth_tfa(server: &mut Server) -> Mock {
    server
        .mock("POST", "/auth/v4/2fa")
        .match_auth()
        .with_status(200)
        .create()
}

/// Mock user info request.
pub fn user_info(server: &mut Server) -> Mock {
    server
        .mock("GET", "/core/v4/users")
        .match_auth()
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(USER_INFO_RESPONSE)
        .create()
}

/// Mock login flow.
///
/// Set `tfa` to true for two factor authentication.
pub fn login_flow(server: &mut Server, tfa: bool) -> Vec<Mock> {
    let mut mocks = Vec::with_capacity(4);
    mocks.push(auth_info(server));
    mocks.push(auth_response(server, tfa));
    if tfa {
        mocks.push(auth_tfa(server))
    }
    mocks.push(user_info(server));
    mocks
}

pub fn auth_refresh(server: &mut Server) -> Mock {
    server
        .mock("POST", "/auth/v4/refresh")
        .match_version()
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(REFRESH_RESPONSE)
        .create()
}

/// TFA Code
pub const TFA_CODE: &str = "012345";

/// Session UID value for mocked requests.
pub const SESSION_UID: &str = "4e9c0760-1660-4327-abd5-308c80173e34";

/// User ID for mocked requests.
pub const USER_ID: &str = "da86bae8-4c0f-4399-8edb-b959bc43eb82";

/// Initial access token for mocked requests.
pub const ACCESS_TOKEN: &str = "110de98b-52cb-4861-9aa9-459b9f8dbc9f";

/// Initial refresh token for mocked requests.
pub const REFRESH_TOKEN: &str = "5bd14ae7-511d-456a-b362-93eb0e58bed3";

/// Post auth refresh access token for mocked requests.
pub const POST_REFRESH_ACCESS_TOKEN: &str = "562f7ab5-d487-4b50-bfb6-c6bb61d1248e";
/// Post auth refresh token for mocked requests.
pub const POST_REFRESH_REFRESH_TOKEN: &str = "8f36f477-a158-406b-bcc9-4eb8d6509358";

const AUTHORIZATION_HEADER_KEY: &str = "authorization";

const AUTHORIZATION_HEADER_VALUE: &str = "Bearer 110de98b-52cb-4861-9aa9-459b9f8dbc9f";
const AUTHORIZATION_HEADER_POST_REFRESH_VALUE: &str = "Bearer 562f7ab5-d487-4b50-bfb6-c6bb61d1248e";

const AUTH_INFO_RESPONSE: &str = r#"
{
  "Version": 4,
  "Modulus": "-----BEGIN PGP SIGNED MESSAGE-----\r\nHash: SHA512\r\n\r\n+88jb48lF5TyDBveyHZ7QhSvtc4V3pN8/eQW6kk6ok2egy4lr5Wz9h8iZP3erN9lReSx1Lk+WsLu1b3soDhXX/twTCUhxYwjS8r983aEshZJJq7p5tNroQ5pzrZMbK8Oszjajgdg2YzcMcaJqb9+Doi7egj/esUQ+Q7BWdxeK77Wafj9v7PiW6Ozx6ulppu1mZ+YGnXSXJsl1Cl4nPm7PNkgj4BQT3HLrxakh7Xc3agmepRKO/1jLaOBU/oO17URbA5rwh/ZlAOqEAKH5vJ+hA2acM3Bwsa/K8I/jWicxOoaLZ4RZFpLYvOxGbb4DggR2Ri/C6tNyeEQQKAtxpeV5g==\r\n-----BEGIN PGP SIGNATURE-----\nVersion: GopenPGP 2.7.1\nComment: https://gopenpgp.org\n\nwl4EARYIABAFAlwB1j0JEDUFhcTpUY8mAAD61gEAo0Uds/t3Fqwq55nOTHlCQxj5\nQ4Ff30YooWIBzvRFtMcA/1LrPUlo++7235+G4JBFJlCw4X4dyTEvhvy7DLwA/YAJ\n=j5fA\n-----END PGP SIGNATURE-----",
  "ServerEphemeral": "Qs2uxScUg4YF/Ri8uHQqPwEAicaXjSNyJQ64XX9HdsaOQCQ3HzfRsI2hI13WHwroPQsEIxJleHzm4vPl9fVU8vkPvTkx8Sy4biL68+lewItaVSo7bXyNXIK8MXA7LjgKxORr77HORtpPTfWvThG7ILAwr/dUDM2lymXHOEnNg82PcRTA6ZSdPtbRPqtrhYvHLeI50AFGIZ0C4o1wZGhVpiyQ/UyOZubI8RRnpErIMB3oZOax1lL7heM4rf7wk3G1x1rkVaXPnSVl5BLCSqtRSpSDXMIeQIALnr9Wn7TLAYQh1KaYHd96fVhQl8zjWTIFfdbppH+nbz3aH/nYEVigSA==",
  "Salt": "9/HTvPnHq7p3YQ3GOLcnxw==",
  "SRPSession": "3fdf84cb-6230-47d1-9668-626b09c4e21c",
  "2FA": {
    "Enabled": 0,
    "FIDO2": {
      "AuthenticationOptions": null,
      "RegisteredKeys": null
    }
  }
}
"#;
const AUTH_RESPONSE: &str = r#"
{
  "UserID": "da86bae8-4c0f-4399-8edb-b959bc43eb82",
  "UID": "4e9c0760-1660-4327-abd5-308c80173e34",
  "AccessToken": "110de98b-52cb-4861-9aa9-459b9f8dbc9f",
  "RefreshToken": "5bd14ae7-511d-456a-b362-93eb0e58bed3",
  "ServerProof": "jVIq126cdeHgnWmlDwkg6UIV0t1R4dZVI0z6oVVzbSZ+RJ7X7SzkEUGAlRtqblWUWMESBs/dDLBUBV6tfcNEdof42US7bjVwU/ENec3ZWKIPe2U9D1COJmMU8thP7MBLGEMmjhrVQMTtTNvFBhjLhzX2fcuYNj/pMxcg5OueeRETGpRrPUzdLYcv7vYWNG033GDI7keuLQORUSHnMfMzb+Yk8aSZ7L48uE2g1UD1L63lCVa5KNP08YwUJUYwGyFGPbt2995cQjWeoHfEA//Z/F/ji2IZXuHihXMhAPYGpKrMGAgjT/0OCp08oiiyV5E/5+O5PKVjY+WMRfpV9w4xzQ==",
  "Scope": "",
  "2FA": {
    "Enabled": 0,
    "FIDO2": {
      "AuthenticationOptions": null,
      "RegisteredKeys": null
    }
  },
  "PasswordMode": 1
}
"#;

const AUTH_RESPONSE_TFA: &str = r#"
{
  "UserID": "da86bae8-4c0f-4399-8edb-b959bc43eb82",
  "UID": "4e9c0760-1660-4327-abd5-308c80173e34",
  "AccessToken": "110de98b-52cb-4861-9aa9-459b9f8dbc9f",
  "RefreshToken": "5bd14ae7-511d-456a-b362-93eb0e58bed3",
  "ServerProof": "jVIq126cdeHgnWmlDwkg6UIV0t1R4dZVI0z6oVVzbSZ+RJ7X7SzkEUGAlRtqblWUWMESBs/dDLBUBV6tfcNEdof42US7bjVwU/ENec3ZWKIPe2U9D1COJmMU8thP7MBLGEMmjhrVQMTtTNvFBhjLhzX2fcuYNj/pMxcg5OueeRETGpRrPUzdLYcv7vYWNG033GDI7keuLQORUSHnMfMzb+Yk8aSZ7L48uE2g1UD1L63lCVa5KNP08YwUJUYwGyFGPbt2995cQjWeoHfEA//Z/F/ji2IZXuHihXMhAPYGpKrMGAgjT/0OCp08oiiyV5E/5+O5PKVjY+WMRfpV9w4xzQ==",
  "Scope": "",
  "2FA": {
    "Enabled": 1,
    "FIDO2": {
      "AuthenticationOptions": null,
      "RegisteredKeys": null
    }
  },
  "PasswordMode": 1
}
"#;

const USER_INFO_RESPONSE: &str = r#"
{
  "User": {
    "ID": "da86bae8-4c0f-4399-8edb-b959bc43eb82",
    "Name": "foo@proton.me",
    "DisplayName": "foo@proton.me",
    "Email": "foo@proton.me",
    "Keys": [
      {
        "PrivateKey": "-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GopenPGP 2.7.1\nComment: https://gopenpgp.org\n\n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\njGedwfbfYQXLvTFj5LZ66RpuO0/RcA==\n=xGZd\n-----END PGP PRIVATE KEY BLOCK-----",
        "ID": "ba89f821-55e6-4320-8760-1031edbaf053",
        "Token": "",
        "Signature": "",
        "Primary": 1,
        "Active": 1,
        "Flags": 0
      }
    ],
    "UsedSpace": 0,
    "MaxSpace": 0,
    "MaxUpload": 0,
    "Credit": 0,
    "Currency": ""
  }
}
"#;

const REFRESH_RESPONSE: &str = r#"
{
  "UserID": "da86bae8-4c0f-4399-8edb-b959bc43eb82",
  "UID": "4e9c0760-1660-4327-abd5-308c80173e34",
  "AccessToken": "562f7ab5-d487-4b50-bfb6-c6bb61d1248e",
  "RefreshToken": "8f36f477-a158-406b-bcc9-4eb8d6509358",
  "ServerProof": "",
  "Scope": "",
  "2FA": {
    "Enabled": 0,
    "FIDO2": {
      "AuthenticationOptions": null,
      "RegisteredKeys": null
    }
  },
  "PasswordMode": 1
}
"#;
