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
        // Scoped so the `MutexGuard` provably drops before the `.await` below --
        // `std::sync::MutexGuard` isn't `Send`-across-await-safe, so it must not
        // still be in scope (even briefly) when we hit an await point.
        let cached = {
            let token = self.current_token.lock().unwrap();
            match &*token {
                Some(tok) if !tok.is_expired() => Some(tok.access_token.clone()),
                _ => None,
            }
        };
        if let Some(access_token) = cached {
            return Ok(access_token);
        }

        // Token expired or not present, refresh it
        let new_token = self.refresh_token().await?;
        let access_token = new_token.access_token.clone();

        *self.current_token.lock().unwrap() = Some(new_token);
        Ok(access_token)
    }

    /// Refresh the OAuth token via a real client-credentials grant POST to
    /// `token_url` (RFC 6749 section 4.4) -- `grant_type=client_credentials`,
    /// `client_id`, `client_secret`, and `scope` if configured.
    async fn refresh_token(&self) -> Result<OAuthToken, AdapterError> {
        if self.client_id.is_empty() || self.client_secret.is_empty() {
            return Err(AdapterError::AuthenticationFailed(
                "Missing OAuth credentials".to_string(),
            ));
        }

        let mut form = vec![
            ("grant_type", "client_credentials"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
        ];
        if let Some(scope) = &self.scope {
            form.push(("scope", scope.as_str()));
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&self.token_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::AuthenticationFailed(format!(
                "token endpoint returned HTTP {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            AdapterError::AuthenticationFailed(format!("invalid token response: {e}"))
        })?;

        let access_token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdapterError::AuthenticationFailed(
                    "token response missing access_token".to_string(),
                )
            })?
            .to_string();
        let token_type = body
            .get("token_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Bearer")
            .to_string();
        let expires_in = body
            .get("expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(3600);

        Ok(OAuthToken {
            access_token,
            token_type,
            expires_in,
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
        let mgr =
            OAuthManager::new("id", "secret", "https://example.com/token").with_scope("read write");

        assert_eq!(mgr.scope, Some("read write".to_string()));
    }

    #[tokio::test]
    async fn missing_credentials_are_rejected_without_a_network_call() {
        let mgr = OAuthManager::new("", "", "https://example.com/token");
        let result = mgr.refresh_token().await;
        assert!(result.is_err());
    }

    // -- Real HTTP tests against a local mock server -------------------------

    #[tokio::test]
    async fn refresh_token_performs_a_real_client_credentials_post() {
        use crate::testing::MockHttpServer;

        let server = MockHttpServer::start(
            200,
            r#"{"access_token":"tok_real_123","token_type":"Bearer","expires_in":1800}"#,
        );
        let mgr = OAuthManager::new(
            "client_id",
            "client_secret",
            format!("{}/token", server.base_url),
        )
        .with_scope("read write");

        let token = mgr.refresh_token().await.unwrap();
        assert_eq!(token.access_token, "tok_real_123");
        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.expires_in, 1800);

        let req = server.last_request().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/token");
        assert!(req.body.contains("grant_type=client_credentials"));
        assert!(req.body.contains("client_id=client_id"));
        assert!(req.body.contains("scope=read+write") || req.body.contains("scope=read%20write"));
    }

    #[tokio::test]
    async fn get_token_caches_the_real_token_and_does_not_refetch_while_valid() {
        use crate::testing::MockHttpServer;

        let server = MockHttpServer::start(
            200,
            r#"{"access_token":"tok_cached","token_type":"Bearer","expires_in":3600}"#,
        );
        let mgr = OAuthManager::new(
            "client_id",
            "client_secret",
            format!("{}/token", server.base_url),
        );

        let token1 = mgr.get_token().await.unwrap();
        let token2 = mgr.get_token().await.unwrap();

        assert_eq!(token1, token2);
        assert_eq!(
            server.requests().len(),
            1,
            "second get_token() must reuse the cached token, not refetch"
        );
    }

    #[tokio::test]
    async fn refresh_token_surfaces_a_real_http_error() {
        use crate::testing::MockHttpServer;

        let server = MockHttpServer::start(401, r#"{"error":"invalid_client"}"#);
        let mgr = OAuthManager::new(
            "client_id",
            "client_secret",
            format!("{}/token", server.base_url),
        );

        let result = mgr.refresh_token().await;
        assert!(result.is_err());
    }
}
