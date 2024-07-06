use crate::clientv2::Session;
use crate::http;
use crate::http::Sequence;

#[derive(Debug)]
pub struct TotpSession(pub(super) Session);

impl TotpSession {
    pub fn submit_totp<'a>(
        &'a self,
        code: &'a str,
    ) -> impl Sequence<Output = Session, Error = http::Error> + 'a {
        let auth = self.0.user_auth.clone();
        self.0
            .submit_totp(code)
            .map(move |_| Ok(Session { user_auth: auth }))
    }

    pub fn logout(&self) -> impl Sequence<Output = ()> + '_ {
        self.0.logout()
    }
}
