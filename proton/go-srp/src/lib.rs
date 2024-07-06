//! Simple bindings for the [go-srp](https://github.com/ProtonMail/go-srp) repository.
//! Currently on the client side proof calcultion is exposed by the bindings as it is the only
//! requirement for an account to log in
//!
//! # Building
//! In order to build this library you need to have go v1.19 or higher installed on your system.
//!
//! # Safety
//! This library needs unsafe to access the C interface exposed by the go library
//!

mod go;
use crate::go::*;

use base64::Engine;
use std::ffi::c_void;
use std::mem::MaybeUninit;

/// Client SRP Auth information.
pub struct SRPAuth {
    pub client_proof: String,
    pub client_ephemeral: String,
    pub expected_server_proof: String,
}

impl SRPAuth {
    /// Creates new Auth from strings input. Salt and server ephemeral are in
    /// base64 format. Modulus is base64 with signature attached. The signature is
    /// verified against server key. The version controls password hash algorithm.
    ///
    /// Parameters:
    ///     version: The *x* component of the vector.
    ///     username: The *y* component of the vector.
    ///     password: The *z* component of the vector.
    ///     salt: The std-base64 formatted salt
    pub fn generate(
        username: &str,
        password: &str,
        version: i64,
        salt: &str,
        modulus: &str,
        server_ephemeral: &str,
    ) -> Result<Self, String> {
        let username = SafeGoString::new(username);
        let modulus = SafeGoString::new(modulus);
        let server_ephemeral = SafeGoString::new(server_ephemeral);
        let salt = SafeGoString::new(salt);

        unsafe {
            let mut result = MaybeUninit::<SRPAuthResult>::zeroed().assume_init();

            let error = SRPAuth(
                username.as_go_string(),
                GoSlice {
                    data: password.as_ptr() as *mut c_void,
                    len: password.len() as GoInt,
                    cap: password.len() as GoInt,
                },
                version as GoInt,
                salt.as_go_string(),
                modulus.as_go_string(),
                server_ephemeral.as_go_string(),
                &mut result,
            );

            if !error.is_null() {
                return Err(OwnedCStr::new(error).to_string());
            }

            let client_proof = CBytes::new(result.client_proof, result.client_proof_len);
            let client_ephemeral =
                CBytes::new(result.client_ephemeral, result.client_ephemeral_len);
            let expected_server_proof = CBytes::new(
                result.expected_server_proof,
                result.expected_server_proof_len,
            );

            let encoder = base64::engine::GeneralPurpose::new(
                &base64::alphabet::STANDARD,
                base64::engine::general_purpose::PAD,
            );

            let b64_client_ephemeral = encoder.encode(client_ephemeral);
            let b64_client_proof = encoder.encode(client_proof);
            let b64_server_proof = encoder.encode(expected_server_proof);

            Ok(Self {
                client_proof: b64_client_proof,
                client_ephemeral: b64_client_ephemeral,
                expected_server_proof: b64_server_proof,
            })
        }
    }
}

#[test]
fn test_srp_call() {
    let version = 4;
    let username = "Cyb3rReaper";
    let password = "123";
    let salt = "CGhrAMJla9YHGQ==";
    let signed_modulus = "-----BEGIN PGP SIGNED MESSAGE-----\nHash: SHA256\n\no4ycZ14/7LfHkuSKWNlpQEh6bwLMVKvo0MFqVq9wHXwkZ/zMcqYaVhqNvLyDB0WY5Uv/Bo23JQsox52lM+4jPydw9/A9saAj8erLCc3ZaZHxOl/a8tlYTq7FeDrbhSSgivwTKJ5Y9otla/U8FATZBxqi7nqDihS5/7x/yK3VRnEsBG1i5DcY1UQK3KD9i9v7N2QTuGFYnRCv0MFsHzrQZWvUa1NsUhozU5PSV5s7hZkb/p6J3B9ybD6+LzuLS9fyLMcVdxzn2WUXG7JLeBbqsoECUfq9KP2waTzVLELOenWUV1wbioceJsaiP97ViwNJdnKx1ICoYu2c+z8ctVcqlw==\n-----BEGIN PGP SIGNATURE-----\nVersion: ProtonMail\nComment: https://protonmail.com\n\nwl4EARYIABAFAlwB1j0JEDUFhcTpUY8mAAB02wD5AOhMNS/K6/nvaeRhTr5n\niDGMalQccYlb58XzUEhqf3sBAOcTsz0fP3PVdMQYBbqcBl9Y6LGIG9DF4B4H\nZeLCoyYN\n=cAxM\n-----END PGP SIGNATURE-----\n";
    let server_ephemeral = "vl0zIXo4bLPtYVoy3kIvhWQx3ObPMYTY0c5/TFHlmwgBW6Hz/p2XDJdDykF3rBfwrSUD4tfs1YRCfgGfvxegCIQhL419OPYgA+ApXUuS2ni86AXUfjPnvJju/inYQxER8nzEhM8DZYAiNM44qeepmXGrHmwjXAMzyaggqxmkTq4v+seKntFE5oH7iIFacgP52wnV/p6OLOMNS4t/vZ3haKaoEVoFyCVVoTJ/OVPp1ZoUovOoxwDvUAOjSEgswenR96xT+4CsPz9Dm+yF/bDugcWGQ4KB8KEzBrO0PqmCQWMYOKaILegtgTjg08eQTvGylSEZmbTeVzoPe/THqh2bJw==";

    let _ = SRPAuth::generate(
        username,
        password,
        version,
        salt,
        signed_modulus,
        server_ephemeral,
    )
    .unwrap();
}
