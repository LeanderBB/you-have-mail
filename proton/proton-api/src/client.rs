use crate::session::DEFAULT_HOST_URL;
use you_have_mail_http::{Client, ClientBuilder};

pub trait ProtonExtension {
    /// Prepare a client builder for the default proton server.
    fn proton_client() -> ClientBuilder;
}

impl ProtonExtension for Client {
    fn proton_client() -> ClientBuilder {
        // This should never fail.
        let base_url = you_have_mail_http::url::Url::parse(DEFAULT_HOST_URL).unwrap();
        Client::builder(base_url)
    }
}
