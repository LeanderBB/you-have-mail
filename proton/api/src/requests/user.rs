use crate::domain::User;
use crate::http;
use crate::http::{JsonResponse, RequestData};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserInfoResponse {
    pub user: User,
}

pub struct UserInfoRequest {}

impl http::RequestDesc for UserInfoRequest {
    type Output = UserInfoResponse;
    type Response = JsonResponse<Self::Output>;

    fn build(&self) -> RequestData {
        RequestData::new(http::Method::Get, "core/v4/users")
    }
}
