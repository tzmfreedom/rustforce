use crate::response::{ErrorResponse, TokenErrorResponse};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotLoggedIn,
    TokenError(TokenErrorResponse),
    HTTPError(String),
    DeserializeError(String),
    ErrorResponses(Vec<ErrorResponse>),
    DescribeError(ErrorResponse),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        match self {
            Error::NotLoggedIn => write!(f, "Not logged in"),
            Error::TokenError(resp) => write!(f, "Invalid token {:?}", resp),
            Error::HTTPError(resp) => write!(f, "HTTP request to Salesforce failed {}", resp),
            Error::DeserializeError(resp) => write!(f, "Could not deserialize response {}", resp),
            Error::ErrorResponses(resp) => write!(f, "Error response from Salesforce {:?}", resp),
            Error::DescribeError(resp) => write!(f, "Error completing describe {:?}", resp),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::HTTPError(e.to_string())
    }
}

impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Self {
        Error::HTTPError(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::DeserializeError(e.to_string())
    }
}
