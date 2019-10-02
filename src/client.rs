extern crate reqwest;

use reqwest::header::HeaderMap;
use reqwest::header::{CONTENT_TYPE};
use std::collections::HashMap;
use std::io::Read;
use std::fmt::Error;
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

pub struct Client {
    clientId: String,
    clientSecret: String,
    loginEndpoint: String,
    accessToken: Option<String>,
    refleshToken: Option<String>,
}

impl Client {
    pub fn new(clientId: &str, clientSecret: &str) -> Client {
        return Client {
            clientId: clientId.to_string(),
            clientSecret: clientSecret.to_string(),
            loginEndpoint: "https://login.salesforce.com".to_string(),
            accessToken: None,
            refleshToken: None,
        }
    }

    pub fn loginWithCredential(&mut self, username: &str, password: &str) {
        let tokenUrl = format!("{}/services/oauth2/token", "https://login.salesforce.com");
        let params = [
            ("grant_type", "password"),
            ("client_id", self.clientId.as_str()),
            ("client_secret", self.clientSecret.as_str()),
            ("username", username),
            ("password", password),
        ];
        let mut headers = HeaderMap::new();
        let client = reqwest::Client::new();
        let mut res: TokenResponse = client.post(tokenUrl.as_str())
            .headers(headers)
            .form(&params)
            .send()
            .unwrap()
            .json()
            .unwrap();
        println!("{:?}", res);
    }
}
