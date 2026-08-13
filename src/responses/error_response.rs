use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub message: String,
    pub error_code: String,
    pub fields: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize() {
        let json_str = json!({
            "message": "Record not found",
            "errorCode": "NOT_FOUND",
            "fields": ["Id"]
        })
        .to_string();

        let resp: ErrorResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(resp.message, "Record not found");
        assert_eq!(resp.error_code, "NOT_FOUND");
        assert_eq!(resp.fields, Some(vec!["Id".to_string()]));
    }

    #[test]
    fn test_without_fields() {
        let json_str = json!({
            "message": "Error",
            "errorCode": "ERR"
        })
        .to_string();

        let resp: ErrorResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(resp.fields, None);
    }

    #[test]
    fn test_default() {
        let resp = ErrorResponse::default();
        assert_eq!(resp.message, "");
        assert_eq!(resp.error_code, "");
        assert_eq!(resp.fields, None);
    }
}
