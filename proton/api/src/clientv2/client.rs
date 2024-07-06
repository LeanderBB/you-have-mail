use crate::http::{Error, RequestDesc, Sequence};
use crate::requests::{CaptchaRequest, Ping};

pub fn ping() -> impl Sequence<Output = (), Error = Error> {
    Ping.to_request()
}

pub fn captcha_get(token: &str, force_web: bool) -> impl Sequence<Output = String, Error = Error> {
    CaptchaRequest::new(token, force_web).to_request()
}
