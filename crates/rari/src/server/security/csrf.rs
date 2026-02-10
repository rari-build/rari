use crate::error::RariError;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const TOKEN_EXPIRATION_SECONDS: u64 = 3600;

pub struct CsrfTokenManager {
    secret: Vec<u8>,
    expiration_seconds: u64,
}

impl CsrfTokenManager {
    pub fn new(secret: Vec<u8>) -> Self {
        Self { secret, expiration_seconds: TOKEN_EXPIRATION_SECONDS }
    }

    pub fn new_with_random_secret() -> Self {
        use rand::RngExt;
        let mut rng = rand::rng();
        let secret: Vec<u8> = (0..32).map(|_| rng.r#random()).collect();
        Self::new(secret)
    }

    pub fn generate_token(&self) -> String {
        let timestamp = self.current_timestamp();
        let signature = self.sign_timestamp(timestamp);
        format!("{}:{}", timestamp, signature)
    }

    pub fn validate_token(&self, token: &str) -> Result<(), RariError> {
        let parts: Vec<&str> = token.split(':').collect();
        if parts.len() != 2 {
            return Err(RariError::bad_request("Invalid CSRF token format"));
        }

        let timestamp_str = parts[0];
        let provided_signature = parts[1];

        let timestamp: u64 = timestamp_str
            .parse()
            .map_err(|_| RariError::bad_request("Invalid CSRF token timestamp"))?;

        let current_time = self.current_timestamp();
        if current_time > timestamp + self.expiration_seconds {
            return Err(RariError::bad_request("CSRF token expired"));
        }

        if timestamp > current_time + 60 {
            return Err(RariError::bad_request("CSRF token timestamp is in the future"));
        }

        let expected_signature = self.sign_timestamp(timestamp);
        if !self.constant_time_compare(&expected_signature, provided_signature) {
            return Err(RariError::bad_request("Invalid CSRF token signature"));
        }

        Ok(())
    }

    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time should be after UNIX_EPOCH")
            .as_secs()
    }

    fn sign_timestamp(&self, timestamp: u64) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC can take key of any size");
        mac.update(timestamp.to_string().as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    fn constant_time_compare(&self, a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let a_bytes = a.as_bytes();
        let b_bytes = b.as_bytes();
        let mut result = 0u8;
        for i in 0..a_bytes.len() {
            result |= a_bytes[i] ^ b_bytes[i];
        }
        result == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn create_test_manager() -> CsrfTokenManager {
        CsrfTokenManager::new(b"test-secret-key-32-bytes-long!!".to_vec())
    }

    #[test]
    fn test_generate_and_validate_token() {
        let manager = create_test_manager();
        let token = manager.generate_token();
        assert!(manager.validate_token(&token).is_ok());
    }

    #[test]
    fn test_invalid_token_format() {
        let manager = create_test_manager();
        assert!(manager.validate_token("invalid").is_err());
        assert!(manager.validate_token("").is_err());
        assert!(manager.validate_token("a:b:c").is_err());
    }

    #[test]
    fn test_invalid_signature() {
        let manager = create_test_manager();
        let timestamp = manager.current_timestamp();
        let invalid_token = format!("{}:invalidsignature", timestamp);
        assert!(manager.validate_token(&invalid_token).is_err());
    }

    #[test]
    fn test_expired_token() {
        let manager = CsrfTokenManager {
            secret: b"test-secret-key-32-bytes-long!!".to_vec(),
            expiration_seconds: 1,
        };
        let token = manager.generate_token();
        thread::sleep(Duration::from_secs(2));
        assert!(manager.validate_token(&token).is_err());
    }

    #[test]
    fn test_future_timestamp_rejected() {
        let manager = create_test_manager();
        let future_timestamp = manager.current_timestamp() + 120;
        let signature = manager.sign_timestamp(future_timestamp);
        let token = format!("{}:{}", future_timestamp, signature);
        assert!(manager.validate_token(&token).is_err());
    }

    #[test]
    fn test_tampered_timestamp() {
        let manager = create_test_manager();
        let token = manager.generate_token();
        let parts: Vec<&str> = token.split(':').collect();
        let signature = parts[1];

        let tampered_timestamp = manager.current_timestamp() - 100;
        let tampered_token = format!("{}:{}", tampered_timestamp, signature);
        assert!(manager.validate_token(&tampered_token).is_err());
    }

    #[test]
    fn test_different_secrets_produce_different_tokens() {
        let manager1 = CsrfTokenManager::new(b"secret1-must-be-32-bytes-long!!".to_vec());
        let manager2 = CsrfTokenManager::new(b"secret2-must-be-32-bytes-long!!".to_vec());

        let token1 = manager1.generate_token();
        assert!(manager2.validate_token(&token1).is_err());
    }

    #[test]
    fn test_constant_time_compare() {
        let manager = create_test_manager();
        assert!(manager.constant_time_compare("abc", "abc"));
        assert!(!manager.constant_time_compare("abc", "abd"));
        assert!(!manager.constant_time_compare("abc", "ab"));
        assert!(!manager.constant_time_compare("abc", "abcd"));
    }

    #[test]
    fn test_random_secret_generation() {
        let manager = CsrfTokenManager::new_with_random_secret();
        let token = manager.generate_token();
        assert!(manager.validate_token(&token).is_ok());
    }

    #[test]
    fn test_token_format() {
        let manager = create_test_manager();
        let token = manager.generate_token();
        let parts: Vec<&str> = token.split(':').collect();
        assert_eq!(parts.len(), 2);

        assert!(parts[0].parse::<u64>().is_ok());

        assert_eq!(parts[1].len(), 64);
        assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()));
    }
}
