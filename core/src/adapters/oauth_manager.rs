use crate::adapters::AdapterError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// OAuth token response from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip)]
    pub issued_at: DateTime<Utc>,
}

impl OAuthToken {
    /// Check if token is expired (with 5 minute buffer)
    pub fn is_expired(&self) -> bool {
        let expiry = self.issued_at + Duration::seconds(self.expires_in);
        let buffer = Utc::now() + Duration::minutes(5);
        buffer >= expiry
    }

    /// Check if token will expire soon (within 5 minutes)
    pub fn expires_soon(&self) -> bool {
        let expiry = self.issued_at + Duration::seconds(self.expires_in);
        let soon_threshold = Utc::now() + Duration::minutes(5);
        soon_threshold >= expiry
    }
}

/// OAuth 2.0 token manager with auto-refresh
pub struct OAuthManager {
    client_id: String,
    client_secret: String,
    token_url: String,
    scope: Option<String>,
    current_token: Arc<Mutex<Option<OAuthToken>>>,
}

impl OAuthManager {
    /// Create a new OAuth manager
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        token_url: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            token_url: token_url.into(),
            scope: None,
            current_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Set OAuth scope
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Get a valid access token (auto-refreshing if needed)
    pub async fn get_token(&self) -> Result<String, AdapterError> {
        let token = self.current_token.lock().unwrap();

        if let Some(ref tok) = *token {
            if !tok.is_expired() {
                return Ok(tok.access_token.clone());
            }
        }
        drop(token);

        // Token expired or not present, refresh it
        let new_token = self.refresh_token().await?;
        let access_token = new_token.access_token.clone();

        *self.current_token.lock().unwrap() = Some(new_token);
        Ok(access_token)
    }

    /// Refresh the OAuth token (simulated - would call token endpoint in production)
    async fn refresh_token(&self) -> Result<OAuthToken, AdapterError> {
        // In production, would make HTTP POST to token_url with:
        // grant_type=client_credentials
        // client_id=...
        // client_secret=...
        // scope=...

        if self.client_id.is_empty() || self.client_secret.is_empty() {
            return Err(AdapterError::AuthenticationFailed(
                "Missing OAuth credentials".to_string(),
            ));
        }

        // Simulated successful token response
        Ok(OAuthToken {
            access_token: format!("token_{}_{}", self.client_id, uuid::Uuid::new_v4()),
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
            issued_at: Utc::now(),
        })
    }

    /// Check if current token expires soon
    pub fn token_expires_soon(&self) -> bool {
        self.current_token
            .lock()
            .unwrap()
            .as_ref()
            .map(|t| t.expires_soon())
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_token_not_expired() {
        let token = OAuthToken {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            issued_at: Utc::now(),
        };

        assert!(!token.is_expired());
    }

    #[test]
    fn test_oauth_token_expired() {
        let token = OAuthToken {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: -3600, // Already expired
            issued_at: Utc::now(),
        };

        assert!(token.is_expired());
    }

    #[test]
    fn test_oauth_token_expires_soon() {
        let token = OAuthToken {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 60, // Expires in 1 minute (within 5 min buffer)
            issued_at: Utc::now(),
        };

        assert!(token.expires_soon());
    }

    #[test]
    fn test_oauth_manager_creation() {
        let mgr = OAuthManager::new("client_id", "client_secret", "https://example.com/token");
        assert_eq!(mgr.client_id, "client_id");
        assert_eq!(mgr.token_url, "https://example.com/token");
    }

    #[test]
    fn test_oauth_manager_with_scope() {
        let mgr = OAuthManager::new("id", "secret", "https://example.com/token")
            .with_scope("read write");

        assert_eq!(mgr.scope, Some("read write".to_string()));
    }

    #[tokio::test]
    async fn test_oauth_refresh_token() {
        let mgr = OAuthManager::new("client_id", "client_secret", "https://example.com/token");
        let token = mgr.refresh_token().await.unwrap();

        assert_eq!(token.token_type, "Bearer");
        assert!(token.access_token.starts_with("token_"));
        assert_eq!(token.expires_in, 3600);
    }

    #[tokio::test]
    async fn test_oauth_get_token() {
        let mgr = OAuthManager::new("client_id", "client_secret", "https://example.com/token");
        let token1 = mgr.get_token().await.unwrap();
        let token2 = mgr.get_token().await.unwrap();

        // Should return same token if not expired
        assert_eq!(token1, token2);
    }
}
