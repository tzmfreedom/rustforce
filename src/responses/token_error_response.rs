use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenErrorResponse {
    error: String,
    error_description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize() {
        let json_str = json!({
            "error": "invalid_grant",
            "error_description": "authentication failure"
        })
        .to_string();

        let resp: TokenErrorResponse = serde_json::from_str(&json_str).unwrap();
        let debug = format!("{:?}", resp);
        assert!(debug.contains("invalid_grant"));
    }

    #[test]
    fn test_clone() {
        let resp: TokenErrorResponse = serde_json::from_str(
            r#"{"error":"e","error_description":"d"}"#,
        )
        .unwrap();
        let cloned = resp.clone();
        let debug = format!("{:?}", cloned);
        assert!(debug.contains("e"));
    }
}
