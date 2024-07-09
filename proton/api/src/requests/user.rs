use crate::domain::user::User;
use http::Method;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetUserInfoResponse {
    pub user: User,
}

#[derive(Copy, Clone)]
pub struct GetUserInfoRequest {}

impl http::Request for GetUserInfoRequest {
    type Response = http::JsonResponse<GetUserInfoResponse>;
    const METHOD: Method = Method::Get;

    fn url(&self) -> String {
        "core/v4/users".to_owned()
    }
}
