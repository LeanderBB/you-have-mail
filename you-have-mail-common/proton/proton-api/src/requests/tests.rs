use you_have_mail_http::Method;

#[derive(Copy, Clone)]
pub struct Ping;

impl you_have_mail_http::Request for Ping {
    type Response = you_have_mail_http::NoResponse;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "tests/ping".to_owned()
    }
}
