use std::time::Duration;

use crate::error::RmeterError;
use crate::http::request::SendRequestInput;
use crate::http::response::SendRequestOutput;

/// Wrapper around a reqwest Client with builder-pattern configuration and
/// connection-pool settings.
pub struct HttpClient {
    inner: reqwest::Client,
}

/// Builder for [`HttpClient`].
pub struct HttpClientBuilder {
    timeout: Duration,
    pool_max_idle_per_host: usize,
    pool_idle_timeout: Duration,
    user_agent: String,
    danger_accept_invalid_certs: bool,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 10,
            pool_idle_timeout: Duration::from_secs(90),
            user_agent: format!("rmeter/{}", env!("CARGO_PKG_VERSION")),
            danger_accept_invalid_certs: false,
        }
    }
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn pool_max_idle_per_host(mut self, n: usize) -> Self {
        self.pool_max_idle_per_host = n;
        self
    }

    pub fn pool_idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool_idle_timeout = timeout;
        self
    }

    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    pub fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.danger_accept_invalid_certs = accept;
        self
    }

    pub fn build(self) -> Result<HttpClient, RmeterError> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .pool_max_idle_per_host(self.pool_max_idle_per_host)
            .pool_idle_timeout(self.pool_idle_timeout)
            .user_agent(self.user_agent)
            .danger_accept_invalid_certs(self.danger_accept_invalid_certs)
            .gzip(true)
            .brotli(true)
            .build()?;

        Ok(HttpClient { inner: client })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        HttpClientBuilder::default()
            .build()
            .expect("Default HttpClient should always build successfully")
    }
}

impl HttpClient {
    /// Create a new client with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a builder for customising the client.
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    /// Send a single HTTP request and return the response with timing data.
    pub async fn send(&self, input: &SendRequestInput) -> Result<SendRequestOutput, RmeterError> {
        use std::time::Instant;

        let method = reqwest::Method::from_bytes(input.method.to_string().as_bytes())
            .map_err(|e| RmeterError::Validation(format!("Invalid HTTP method: {e}")))?;

        let mut builder = self.inner.request(method, &input.url);

        // Headers
        for (key, value) in &input.headers {
            builder = builder.header(key, value);
        }

        // Authentication
        if let Some(auth) = &input.auth {
            match auth {
                crate::http::request::Auth::Bearer(token) => {
                    builder = builder.bearer_auth(token);
                }
                crate::http::request::Auth::Basic { username, password } => {
                    builder = builder.basic_auth(username, password.as_deref());
                }
            }
        }

        // Body
        if let Some(body) = &input.body {
            use crate::plan::model::RequestBody;
            match body {
                RequestBody::Json(json_str) => {
                    let value: serde_json::Value = serde_json::from_str(json_str)?;
                    builder = builder.json(&value);
                }
                RequestBody::FormData(pairs) => {
                    let params: Vec<(&str, &str)> =
                        pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                    builder = builder.form(&params);
                }
                RequestBody::Raw(raw) => {
                    builder = builder.body(raw.clone());
                }
                RequestBody::Xml(xml) => {
                    builder = builder
                        .header("Content-Type", "application/xml")
                        .body(xml.clone());
                }
            }
        }

        let start = Instant::now();
        let response = builder.send().await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let status = response.status().as_u16();
        let headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str().ok().map(|v| (k.as_str().to_owned(), v.to_owned()))
            })
            .collect();

        let body_bytes = response.bytes().await?;
        let size_bytes = body_bytes.len() as u64;
        let body = String::from_utf8_lossy(&body_bytes).into_owned();

        Ok(SendRequestOutput {
            status,
            headers,
            body,
            elapsed_ms,
            size_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_client_builds_successfully() {
        let client = HttpClient::new();
        // If we got here, the default client was built without error
        let _ = client;
    }

    #[test]
    fn builder_default_builds_successfully() {
        let client = HttpClientBuilder::default().build();
        assert!(client.is_ok());
    }

    #[test]
    fn builder_with_custom_timeout() {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(60))
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn builder_with_custom_pool_settings() {
        let client = HttpClient::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(120))
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn builder_with_custom_user_agent() {
        let client = HttpClient::builder()
            .user_agent("test-agent/1.0")
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn builder_with_accept_invalid_certs() {
        let client = HttpClient::builder()
            .danger_accept_invalid_certs(true)
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn builder_chaining_all_options() {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(5)
            .pool_idle_timeout(Duration::from_secs(30))
            .user_agent("rmeter-test")
            .danger_accept_invalid_certs(false)
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn builder_returns_http_client_type() {
        let client: HttpClient = HttpClient::builder().build().unwrap();
        let _ = client;
    }

    #[test]
    fn default_builder_has_expected_values() {
        let builder = HttpClientBuilder::default();
        assert_eq!(builder.timeout, Duration::from_secs(30));
        assert_eq!(builder.pool_max_idle_per_host, 10);
        assert_eq!(builder.pool_idle_timeout, Duration::from_secs(90));
        assert!(!builder.danger_accept_invalid_certs);
        assert!(builder.user_agent.starts_with("rmeter/"));
    }
}
