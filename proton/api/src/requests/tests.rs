use crate::http;
use crate::http::RequestData;

pub struct Ping;

impl http::RequestDesc for Ping {
    type Output = ();
    type Response = http::NoResponse;

    fn build(&self) -> RequestData {
        RequestData::new(http::Method::Get, "tests/ping")
    }
}
