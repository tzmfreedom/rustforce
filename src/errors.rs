use std::fmt;
use crate::responses::error_response::ErrorResponse;
use crate::responses::token_error_response::TokenErrorResponse;
use reqwest::header::InvalidHeaderValue;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    NotLoggedIn,
    ConfigError(String),
    TokenError(TokenErrorResponse),
    HttpError(reqwest::Error),
    HeaderError(InvalidHeaderValue),
    DeserializeError(serde_json::Error),
    ErrorResponses(Vec<ErrorResponse>),
    DescribeError(ErrorResponse),
    LoginError(ErrorResponse),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::HttpError(e) => Some(e),
            Error::HeaderError(e) => Some(e),
            Error::DeserializeError(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NotLoggedIn => write!(f, "Not logged in"),
            Error::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Error::TokenError(resp) => write!(f, "Invalid token {:?}", resp),
            Error::HttpError(e) => write!(f, "HTTP request to Salesforce failed: {}", e),
            Error::HeaderError(e) => write!(f, "Invalid header value: {}", e),
            Error::DeserializeError(e) => write!(f, "Could not deserialize response: {}", e),
            Error::ErrorResponses(resp) => write!(f, "Error response from Salesforce {:?}", resp),
            Error::DescribeError(resp) => write!(f, "Error completing describe {:?}", resp),
            Error::LoginError(resp) => write!(f, "Error logging in {:?}", resp),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::HttpError(e)
    }
}

impl From<InvalidHeaderValue> for Error {
    fn from(e: InvalidHeaderValue) -> Self {
        Error::HeaderError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::DeserializeError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn test_display_not_logged_in() {
        let err = Error::NotLoggedIn;
        assert_eq!(format!("{}", err), "Not logged in");
    }

    #[test]
    fn test_display_config_error() {
        let err = Error::ConfigError("client_id is required".to_string());
        assert_eq!(
            format!("{}", err),
            "Configuration error: client_id is required"
        );
    }

    #[test]
    fn test_display_token_error() {
        let token_err: TokenErrorResponse = serde_json::from_str(
            r#"{"error":"invalid_grant","error_description":"bad credentials"}"#,
        )
        .unwrap();
        let err = Error::TokenError(token_err);
        let display = format!("{}", err);
        assert!(display.contains("Invalid token"));
        assert!(display.contains("invalid_grant"));
    }

    #[test]
    fn test_display_deserialize_error() {
        let json_err = serde_json::from_str::<String>("invalid json").unwrap_err();
        let err = Error::DeserializeError(json_err);
        let display = format!("{}", err);
        assert!(display.starts_with("Could not deserialize response: "));
    }

    #[test]
    fn test_display_header_error() {
        let header_err = reqwest::header::HeaderValue::from_str("\0").unwrap_err();
        let err = Error::HeaderError(header_err);
        let display = format!("{}", err);
        assert!(display.starts_with("Invalid header value: "));
    }

    #[test]
    fn test_display_error_responses() {
        let responses = vec![ErrorResponse {
            message: "Record not found".to_string(),
            error_code: "NOT_FOUND".to_string(),
            fields: None,
        }];
        let err = Error::ErrorResponses(responses);
        let display = format!("{}", err);
        assert!(display.contains("Error response from Salesforce"));
        assert!(display.contains("NOT_FOUND"));
    }

    #[test]
    fn test_display_describe_error() {
        let resp = ErrorResponse {
            message: "Cannot describe".to_string(),
            error_code: "INVALID_TYPE".to_string(),
            fields: Some(vec!["Name".to_string()]),
        };
        let err = Error::DescribeError(resp);
        let display = format!("{}", err);
        assert!(display.contains("Error completing describe"));
    }

    #[test]
    fn test_display_login_error() {
        let resp = ErrorResponse {
            message: "Login failed".to_string(),
            error_code: "LOGIN_FAILED".to_string(),
            fields: None,
        };
        let err = Error::LoginError(resp);
        let display = format!("{}", err);
        assert!(display.contains("Error logging in"));
    }

    #[test]
    fn test_error_is_std_error() {
        let err = Error::NotLoggedIn;
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_source_deserialize_error() {
        let json_err = serde_json::from_str::<String>("bad").unwrap_err();
        let err = Error::DeserializeError(json_err);
        assert!(StdError::source(&err).is_some());
    }

    #[test]
    fn test_source_header_error() {
        let header_err = reqwest::header::HeaderValue::from_str("\0").unwrap_err();
        let err = Error::HeaderError(header_err);
        assert!(StdError::source(&err).is_some());
    }

    #[test]
    fn test_source_none_for_other_variants() {
        assert!(StdError::source(&Error::NotLoggedIn).is_none());
        assert!(StdError::source(&Error::ConfigError("test".to_string())).is_none());
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<String>("invalid json").unwrap_err();
        let err: Error = json_err.into();
        match err {
            Error::DeserializeError(_) => {}
            _ => panic!("Expected DeserializeError"),
        }
    }

    #[test]
    fn test_from_invalid_header_value() {
        let header_err = reqwest::header::HeaderValue::from_str("\0").unwrap_err();
        let err: Error = header_err.into();
        match err {
            Error::HeaderError(_) => {}
            _ => panic!("Expected HeaderError"),
        }
    }

    #[test]
    fn test_debug_impl() {
        let err = Error::NotLoggedIn;
        let debug = format!("{:?}", err);
        assert_eq!(debug, "NotLoggedIn");
    }
}
