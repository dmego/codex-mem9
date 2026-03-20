use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Serialize;

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
        self.client
            .post(url)
            .json(payload)
            .send()
            .await
            .context("Mem9 store request failed")?
            .error_for_status()
            .context("Mem9 store returned a non-success status")?;
        Ok(())
    }
}
