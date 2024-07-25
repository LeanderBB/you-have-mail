use http::Method;

#[derive(Copy, Clone)]
pub struct Ping;

impl http::Request for Ping {
    type Response = http::NoResponse;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "tests/ping".to_owned()
    }
}
