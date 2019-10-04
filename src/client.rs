extern crate reqwest;

use reqwest::header::HeaderMap;
use serde::Deserialize;

#[derive(Deserialize)]
#[derive(Debug)]
pub struct TokenResponse {
    id: String,
    issued_at: String,
    access_token: String,
    instance_url: String,
    signature: String,
    token_type: String,
}

#[derive(Debug)]
pub struct Client {
    client_id: String,
    client_secret: String,
    login_endpoint: String,
    access_token: Option<String>,
    reflesh_token: Option<String>,
}

impl Client {
    pub fn new(client_id: String, client_secret: String) -> Client {
        return Client {
            client_id: client_id,
            client_secret: client_secret,
            login_endpoint: "https://login.salesforce.com".to_string(),
            access_token: None,
            reflesh_token: None,
        }
    }

    pub fn login_with_credential(&mut self, username: String, password: String) {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "password"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("username", username.as_str()),
            ("password", password.as_str()),
        ];
        let client = reqwest::Client::new();
        let res: TokenResponse = client.post(token_url.as_str())
            .form(&params)
            .send()
            .unwrap()
            .json()
            .unwrap();
        self.access_token = Some(res.access_token);
    }
}
