use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TokenResponse {
    pub id: String,
    pub issued_at: String,
    pub access_token: String,
    pub instance_url: String,
    pub signature: String,
    pub token_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize() {
        let json_str = json!({
            "access_token": "token123",
            "issued_at": "1234567890",
            "id": "https://login.salesforce.com/id/00Dxx/005xx",
            "instance_url": "https://na1.salesforce.com",
            "signature": "sig",
            "token_type": "Bearer"
        })
        .to_string();

        let resp: TokenResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(resp.access_token, "token123");
        assert_eq!(resp.issued_at, "1234567890");
        assert_eq!(resp.instance_url, "https://na1.salesforce.com");
        assert_eq!(resp.token_type, Some("Bearer".to_string()));
    }

    #[test]
    fn test_without_token_type() {
        let json_str = json!({
            "access_token": "token123",
            "issued_at": "1234567890",
            "id": "id",
            "instance_url": "https://na1.salesforce.com",
            "signature": "sig"
        })
        .to_string();

        let resp: TokenResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(resp.token_type, None);
    }

    #[test]
    fn test_default() {
        let resp = TokenResponse::default();
        assert_eq!(resp.access_token, "");
        assert_eq!(resp.issued_at, "");
        assert_eq!(resp.instance_url, "");
        assert_eq!(resp.token_type, None);
    }

    #[test]
    fn test_clone() {
        let resp = TokenResponse {
            id: "id".to_string(),
            issued_at: "123".to_string(),
            access_token: "tok".to_string(),
            instance_url: "url".to_string(),
            signature: "sig".to_string(),
            token_type: Some("Bearer".to_string()),
        };
        let cloned = resp.clone();
        assert_eq!(cloned.access_token, "tok");
    }

    #[test]
    fn test_serialize() {
        let resp = TokenResponse {
            id: "id".to_string(),
            issued_at: "123".to_string(),
            access_token: "tok".to_string(),
            instance_url: "url".to_string(),
            signature: "sig".to_string(),
            token_type: Some("Bearer".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"access_token\":\"tok\""));
    }
}
