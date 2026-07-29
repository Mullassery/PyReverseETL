/// Credential Management for StatGuardian API
///
/// Securely handles authentication credentials for StatGuardian API access.
/// Supports multiple authentication methods and environment-based credential loading.

use crate::Result;
use serde::{Deserialize, Serialize};

/// Authentication method for StatGuardian API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Bearer token authentication
    Bearer(String),
    /// API Key in header
    ApiKey {
        header_name: String,
        key: String,
    },
    /// Basic authentication
    BasicAuth {
        username: String,
        password: String,
    },
}

/// Credentials for StatGuardian API access
#[derive(Debug, Clone)]
pub struct GovernanceCredentials {
    /// Primary API key
    api_key: String,
    /// Authentication method
    auth_method: AuthMethod,
}

impl GovernanceCredentials {
    /// Create new credentials with bearer token
    pub fn bearer(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            api_key: token.clone(),
            auth_method: AuthMethod::Bearer(token),
        }
    }

    /// Create new credentials with API key header
    pub fn with_api_key(header_name: impl Into<String>, key: impl Into<String>) -> Self {
        let header = header_name.into();
        let key_val = key.into();
        Self {
            api_key: key_val.clone(),
            auth_method: AuthMethod::ApiKey {
                header_name: header,
                key: key_val,
            },
        }
    }

    /// Create new credentials with basic auth
    pub fn basic_auth(username: impl Into<String>, password: impl Into<String>) -> Self {
        let user = username.into();
        let pass = password.into();
        let combined = format!("{}:{}", user, pass);
        Self {
            api_key: combined,
            auth_method: AuthMethod::BasicAuth {
                username: user,
                password: pass,
            },
        }
    }

    /// Load credentials from environment variables
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("STATGUARDIAN_TOKEN")
            .or_else(|_| std::env::var("STATGUARDIAN_API_KEY"))
            .map_err(|_| crate::Error::ConfigError(
                "STATGUARDIAN_TOKEN or STATGUARDIAN_API_KEY not set".to_string()
            ))?;

        Ok(Self::bearer(token))
    }

    /// Get the API key
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get the authentication method
    pub fn auth_method(&self) -> &AuthMethod {
        &self.auth_method
    }

    /// Validate credentials are not empty
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(crate::Error::ConfigError(
                "API key cannot be empty".to_string()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearer_token_creation() {
        let creds = GovernanceCredentials::bearer("test-token-123");
        assert_eq!(creds.api_key(), "test-token-123");

        match creds.auth_method() {
            AuthMethod::Bearer(token) => {
                assert_eq!(token, "test-token-123");
            }
            _ => panic!("Expected Bearer auth method"),
        }
    }

    #[test]
    fn test_api_key_creation() {
        let creds = GovernanceCredentials::with_api_key("X-API-Key", "secret-key-456");
        assert_eq!(creds.api_key(), "secret-key-456");

        match creds.auth_method() {
            AuthMethod::ApiKey { header_name, key } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(key, "secret-key-456");
            }
            _ => panic!("Expected ApiKey auth method"),
        }
    }

    #[test]
    fn test_basic_auth_creation() {
        let creds = GovernanceCredentials::basic_auth("user", "password");
        assert_eq!(creds.api_key(), "user:password");

        match creds.auth_method() {
            AuthMethod::BasicAuth { username, password } => {
                assert_eq!(username, "user");
                assert_eq!(password, "password");
            }
            _ => panic!("Expected BasicAuth auth method"),
        }
    }

    #[test]
    fn test_validate_credentials() {
        let creds = GovernanceCredentials::bearer("valid-token");
        assert!(creds.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_credentials() {
        let creds = GovernanceCredentials::bearer("");
        assert!(creds.validate().is_err());
    }

    #[test]
    fn test_from_env() {
        // This test will only pass if env var is set
        // Skipped by default unless environment is configured
        if let Ok(token) = std::env::var("STATGUARDIAN_TOKEN") {
            let creds = GovernanceCredentials::from_env().unwrap();
            assert_eq!(creds.api_key(), token);
        }
    }
}
