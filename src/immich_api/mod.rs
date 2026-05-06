use crate::config::ApiConfig;
use crate::error::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use urlencoding::encode as urlencode;

#[derive(Clone)]
pub struct ImmichClient {
    client: Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo { pub version: String }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub people: Option<Vec<PersonRef>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonRef {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse { assets: SearchAssets }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchAssets {
    items: Vec<Asset>,
    next_page: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    person_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    taken_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    taken_before: Option<String>,
    #[serde(rename = "type")]
    asset_type: String,
    page: u32,
    size: u32,
    with_people: bool,
}

impl ImmichClient {
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        Ok(Self {
            client,
            base_url: Self::sanitize_base_url(&config.base_url),
            api_key: config.api_key.clone(),
        })
    }

    fn sanitize_base_url(url: &str) -> String {
        let trimmed = url.trim_end_matches('/');
        if trimmed.ends_with("/api") { trimmed.into() } else { format!("{}/api", trimmed) }
    }

    async fn check(resp: reqwest::Response, op: &str) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() { return Ok(resp); }
        let body = resp.text().await.unwrap_or_default();
        Err(match status {
            reqwest::StatusCode::UNAUTHORIZED => Error::ImmichApi(format!("{op}: invalid or expired API key (401)")),
            reqwest::StatusCode::FORBIDDEN    => Error::ImmichApi(format!("{op}: missing permission (403). {body}")),
            _ => Error::ImmichApi(format!("{op}: HTTP {status}. {body}")),
        })
    }

    pub async fn validate_connection(&self) -> Result<ServerInfo> {
        let url = format!("{}/server/about", self.base_url);
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Validate connection").await?;
        Ok(resp.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_base_url() {
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283/api"), "http://x:2283/api");
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283/api/"), "http://x:2283/api");
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283"), "http://x:2283/api");
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283/"), "http://x:2283/api");
    }
}
