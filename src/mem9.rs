use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::Serialize;

const CLIENT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct Mem9Client {
    api_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize)]
pub struct StorePayload {
    pub content: String,
    pub tags: Vec<String>,
    pub source: String,
}

impl Mem9Client {
    pub fn new(api_url: String, api_key: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-API-Key",
            HeaderValue::from_str(&api_key).context("invalid MEM9 API key")?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static(CLIENT_USER_AGENT));
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to build Mem9 HTTP client")?;
        Ok(Self { api_url, client })
    }

    pub async fn store(&self, payload: &StorePayload) -> Result<()> {
        let url = format!(
            "{}/v1alpha2/mem9s/memories",
            self.api_url.trim_end_matches('/')
        );
        let response = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await
            .context("Mem9 store request failed")?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        let body = response.text().await.unwrap_or_default();
        if body.trim().is_empty() {
            bail!("Mem9 store returned a non-success status: HTTP status {status}");
        }

        bail!(
            "Mem9 store returned a non-success status: HTTP status {status}; response body: {}",
            body.trim()
        );
    }
}

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;

    use super::{CLIENT_USER_AGENT, Mem9Client, StorePayload};

    #[tokio::test]
    async fn store_sends_a_user_agent_header() {
        let server = MockServer::start();
        let payload = StorePayload {
            content: "test".to_string(),
            tags: vec!["tag".to_string()],
            source: "codex-mem9:test".to_string(),
        };

        let store_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1alpha2/mem9s/memories")
                .header("x-api-key", "api-key")
                .header("user-agent", CLIENT_USER_AGENT);
            then.status(202)
                .header("content-type", "application/json")
                .body(r#"{"status":"accepted"}"#);
        });

        let client = Mem9Client::new(server.base_url(), "api-key".to_string()).unwrap();
        client.store(&payload).await.unwrap();

        store_mock.assert();
    }

    #[tokio::test]
    async fn store_includes_response_body_in_errors() {
        let server = MockServer::start();
        let payload = StorePayload {
            content: "test".to_string(),
            tags: vec!["tag".to_string()],
            source: "codex-mem9:test".to_string(),
        };

        server.mock(|when, then| {
            when.method(POST).path("/v1alpha2/mem9s/memories");
            then.status(403)
                .header("content-type", "text/plain")
                .body("forbidden by upstream");
        });

        let client = Mem9Client::new(server.base_url(), "api-key".to_string()).unwrap();
        let error = client.store(&payload).await.unwrap_err().to_string();

        assert!(error.contains("403"));
        assert!(error.contains("forbidden by upstream"));
    }
}
