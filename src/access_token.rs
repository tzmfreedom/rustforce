#[derive(Debug, Clone, Default)]
pub struct AccessToken {
    pub token_type: String,
    pub value: String,
    pub issued_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let token = AccessToken::default();
        assert_eq!(token.token_type, "");
        assert_eq!(token.value, "");
        assert_eq!(token.issued_at, "");
    }

    #[test]
    fn test_clone() {
        let token = AccessToken {
            token_type: "Bearer".to_string(),
            value: "abc123".to_string(),
            issued_at: "1234567890".to_string(),
        };
        let cloned = token.clone();
        assert_eq!(cloned.token_type, "Bearer");
        assert_eq!(cloned.value, "abc123");
        assert_eq!(cloned.issued_at, "1234567890");
    }

    #[test]
    fn test_debug() {
        let token = AccessToken {
            token_type: "Bearer".to_string(),
            value: "tok".to_string(),
            issued_at: "123".to_string(),
        };
        let debug = format!("{:?}", token);
        assert!(debug.contains("Bearer"));
        assert!(debug.contains("tok"));
        assert!(debug.contains("123"));
    }
}
