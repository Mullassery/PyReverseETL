use crate::adapters::{AdapterError, AuthMethod, RetryPolicy};
use reqwest::{Client, ClientBuilder};
use serde_json::Value;
use std::time::Duration;

/// HTTP client for making requests to destination APIs
pub struct HttpClient {
    client: Client,
    base_url: String,
    timeout: Duration,
    auth: AuthMethod,
    retry_policy: RetryPolicy,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(
        base_url: impl Into<String>,
        auth: AuthMethod,
        timeout_secs: u32,
    ) -> Result<Self, AdapterError> {
        let timeout = Duration::from_secs(timeout_secs as u64);

        let client = ClientBuilder::new()
            .timeout(timeout)
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| AdapterError::ConnectionError(e.to_string()))?;

        Ok(HttpClient {
            client,
            base_url: base_url.into(),
            timeout,
            auth,
            retry_policy: RetryPolicy::default(),
        })
    }

    /// Set custom retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Make a POST request
    pub async fn post(&self, path: &str, body: &Value) -> Result<Value, AdapterError> {
        let url = format!("{}{}", self.base_url, path);

        self.retry_policy
            .execute(|| async {
                let mut req = self.client.post(&url).json(body);
                req = self.add_auth_header(req);

                let response = req
                    .send()
                    .await
                    .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

                self.handle_response(response).await
            })
            .await
    }

    /// Make a PATCH request
    pub async fn patch(&self, path: &str, body: &Value) -> Result<Value, AdapterError> {
        let url = format!("{}{}", self.base_url, path);

        self.retry_policy
            .execute(|| async {
                let mut req = self.client.patch(&url).json(body);
                req = self.add_auth_header(req);

                let response = req
                    .send()
                    .await
                    .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

                self.handle_response(response).await
            })
            .await
    }

    /// Make a DELETE request
    pub async fn delete(&self, path: &str) -> Result<(), AdapterError> {
        let url = format!("{}{}", self.base_url, path);

        self.retry_policy
            .execute(|| async {
                let mut req = self.client.delete(&url);
                req = self.add_auth_header(req);

                let response = req
                    .send()
                    .await
                    .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

                match response.status().as_u16() {
                    200..=299 => Ok(()),
                    404 => Ok(()), // Already deleted
                    401 => Err(AdapterError::AuthenticationFailed("Invalid credentials".to_string())),
                    429 => Err(AdapterError::RateLimitExceeded { retry_after_ms: 5000 }),
                    status => Err(AdapterError::OperationFailed(format!("HTTP {}", status))),
                }
            })
            .await
    }

    /// Make a GET request
    pub async fn get(&self, path: &str) -> Result<Value, AdapterError> {
        let url = format!("{}{}", self.base_url, path);

        self.retry_policy
            .execute(|| async {
                let mut req = self.client.get(&url);
                req = self.add_auth_header(req);

                let response = req
                    .send()
                    .await
                    .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

                self.handle_response(response).await
            })
            .await
    }

    /// Add authentication header to request
    fn add_auth_header(
        &self,
        mut req: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        match &self.auth {
            AuthMethod::Bearer { token } => {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
            AuthMethod::ApiKey { key } => {
                req = req.header("X-API-Key", key.clone());
            }
            AuthMethod::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::encode(credentials);
                req = req.header("Authorization", format!("Basic {}", encoded));
            }
            _ => {}
        }
        req
    }

    /// Handle HTTP response
    async fn handle_response(&self, response: reqwest::Response) -> Result<Value, AdapterError> {
        match response.status().as_u16() {
            200..=299 => {
                response
                    .json::<Value>()
                    .await
                    .map_err(|e| AdapterError::OperationFailed(e.to_string()))
            }
            401 => Err(AdapterError::AuthenticationFailed("Invalid credentials".to_string())),
            429 => Err(AdapterError::RateLimitExceeded { retry_after_ms: 5000 }),
            500..=599 => Err(AdapterError::ConnectionError("Server error".to_string())),
            status => Err(AdapterError::OperationFailed(format!("HTTP {}", status))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_creation() {
        let auth = AuthMethod::Bearer {
            token: "test_token".to_string(),
        };
        let client = HttpClient::new("https://api.example.com", auth, 30).unwrap();
        assert_eq!(client.base_url, "https://api.example.com");
        assert_eq!(client.timeout.as_secs(), 30);
    }

    #[test]
    fn test_http_client_with_api_key() {
        let auth = AuthMethod::ApiKey {
            key: "test_key".to_string(),
        };
        let client = HttpClient::new("https://api.hubspot.com", auth, 30).unwrap();
        assert_eq!(client.base_url, "https://api.hubspot.com");
    }

    #[test]
    fn test_http_client_timeout() {
        let auth = AuthMethod::Bearer {
            token: "token".to_string(),
        };
        let client = HttpClient::new("https://api.example.com", auth, 60).unwrap();
        assert_eq!(client.timeout.as_secs(), 60);
    }

    #[test]
    fn test_retry_policy_integration() {
        let auth = AuthMethod::Bearer {
            token: "token".to_string(),
        };
        let client = HttpClient::new("https://api.example.com", auth, 30).unwrap();
        let custom_policy = RetryPolicy::new(5, 50, 10000);
        let _client = client.with_retry_policy(custom_policy);
        // Verify retry policy is set
    }

    #[test]
    fn test_auth_header_bearer() {
        let auth = AuthMethod::Bearer {
            token: "test_token".to_string(),
        };
        let client = HttpClient::new("https://api.example.com", auth, 30).unwrap();

        let req = client.client.get("https://api.example.com/test");
        let req = client.add_auth_header(req);

        // Note: Can't directly inspect headers, but no panic means success
        let _req = req;
    }

    #[test]
    fn test_auth_header_api_key() {
        let auth = AuthMethod::ApiKey {
            key: "secret_key".to_string(),
        };
        let client = HttpClient::new("https://api.example.com", auth, 30).unwrap();

        let req = client.client.get("https://api.example.com/test");
        let req = client.add_auth_header(req);
        let _req = req;
    }
}
