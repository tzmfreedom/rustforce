extern crate reqwest;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse<T> {
    pub total_size: i32,
    pub done: bool,
    pub records: Vec<T>,
}

#[derive(Deserialize, Debug)]
pub struct CreateResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, Debug)]
pub struct UpsertResponse {
    create: Option<CreateResponse>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub message: String,
    pub error_code: String,
    pub fields: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub id: String,
    pub issued_at: String,
    pub access_token: String,
    pub instance_url: String,
    pub signature: String,
    pub token_type: String,
}

#[derive(Deserialize, Debug)]
pub struct TokenErrorResponse {
    error: String,
    error_description: String,
}

#[derive(Debug)]
pub struct AccessToken {
    pub token_type: String,
    pub value: String,
    pub issued_at: String,
}
