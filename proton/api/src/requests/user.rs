use crate::domain::User;
use http::Method;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserInfoResponse {
    pub user: User,
}

#[derive(Copy, Clone)]
pub struct UserInfoRequest {}

impl http::Request for UserInfoRequest {
    type Response = http::JsonResponse<UserInfoResponse>;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "core/v4/users".to_owned()
    }
}
