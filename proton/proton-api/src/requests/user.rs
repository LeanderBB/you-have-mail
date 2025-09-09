use crate::domain::user::User;
use serde::Deserialize;
use you_have_mail_http::Method;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetUserInfoResponse {
    pub user: User,
}

#[derive(Copy, Clone)]
pub struct GetUserInfoRequest {}

impl you_have_mail_http::Request for GetUserInfoRequest {
    type Response = you_have_mail_http::JsonResponse<GetUserInfoResponse>;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "core/v4/users".to_owned()
    }
}
