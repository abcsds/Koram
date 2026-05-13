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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub album_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    pub id: String,
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

    pub async fn get_people(&self) -> Result<Vec<Person>> {
        let url = format!("{}/people", self.base_url);
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Get people").await?;
        #[derive(Deserialize)]
        struct R { people: Vec<Person> }
        let r: R = resp.json().await?;
        Ok(r.people)
    }

    pub async fn get_person_statistics(&self, person_id: &str) -> Result<u64> {
        let url = format!("{}/people/{}/statistics", self.base_url, urlencode(person_id));
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Get person statistics").await?;
        let body = resp.text().await?;
        // The Immich docs say `{"assets": N}`, but be tolerant of camelCase variants
        // (`assetCount`, `numberOfPhotos`) seen on older deployments. Pick whichever
        // numeric field is present.
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| Error::ImmichApi(format!("Get person statistics: bad JSON ({}): {}", e, body)))?;
        let n = v.get("assets")
            .or_else(|| v.get("assetCount"))
            .or_else(|| v.get("numberOfPhotos"))
            .and_then(|x| x.as_u64());
        match n {
            Some(n) => Ok(n),
            None => Err(Error::ImmichApi(format!(
                "Get person statistics for {}: no count field in {}", person_id, body
            ))),
        }
    }

    pub async fn get_person_thumbnail(&self, person_id: &str) -> Result<(bytes::Bytes, String)> {
        let url = format!("{}/people/{}/thumbnail", self.base_url, urlencode(person_id));
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Get person thumbnail").await?;
        let ct = resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).unwrap_or("image/jpeg").to_string();
        let bytes = resp.bytes().await?;
        Ok((bytes, ct))
    }

    /// Paginated fetch of every asset that contains `person_id`, optionally filtered by date range.
    /// Returns assets with `with_people=true` so the caller can read every face per asset.
    pub async fn search_person_assets(
        &self,
        person_id: &str,
        taken_after: Option<&str>,
        taken_before: Option<&str>,
    ) -> Result<Vec<Asset>> {
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let params = SearchParams {
                person_ids: Some(vec![person_id.to_string()]),
                taken_after: taken_after.map(String::from),
                taken_before: taken_before.map(String::from),
                asset_type: "IMAGE".into(),
                page,
                size: 100,
                with_people: true,
            };
            let url = format!("{}/search/metadata", self.base_url);
            let resp = self.client.post(&url)
                .header("x-api-key", &self.api_key)
                .json(&params)
                .send().await?;
            let resp = Self::check(resp, "Search assets").await?;
            let r: SearchResponse = resp.json().await?;
            all.extend(r.assets.items);
            if r.assets.next_page.is_none() { break; }
            page += 1;
            if page > 1000 { tracing::warn!("Pagination cap hit"); break; }
        }
        Ok(all)
    }

    pub async fn get_albums(&self) -> Result<Vec<Album>> {
        let url = format!("{}/albums", self.base_url);
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Get albums").await?;
        Ok(resp.json().await?)
    }

    pub async fn create_album(&self, name: &str) -> Result<Album> {
        let url = format!("{}/albums", self.base_url);
        let body = serde_json::json!({ "albumName": name });
        let resp = self.client.post(&url)
            .header("x-api-key", &self.api_key)
            .json(&body).send().await?;
        let resp = Self::check(resp, "Create album").await?;
        Ok(resp.json().await?)
    }

    pub async fn ensure_album(&self, name: &str) -> Result<Album> {
        for a in self.get_albums().await? {
            if a.album_name == name { return Ok(a); }
        }
        self.create_album(name).await
    }

    pub async fn add_assets_to_album(&self, album_id: &str, asset_ids: &[String]) -> Result<()> {
        let url = format!("{}/albums/{}/assets", self.base_url, urlencode(album_id));
        let body = serde_json::json!({ "ids": asset_ids });
        let resp = self.client.put(&url)
            .header("x-api-key", &self.api_key)
            .json(&body).send().await?;
        Self::check(resp, "Add assets to album").await?;
        Ok(())
    }

    /// Upload a PNG asset. Returns the new asset's Immich ID.
    pub async fn upload_asset(
        &self,
        png_bytes: bytes::Bytes,
        device_asset_id: &str,
        file_created_at: &str,
    ) -> Result<UploadResponse> {
        let url = format!("{}/assets", self.base_url);

        let part = reqwest::multipart::Part::bytes(png_bytes.to_vec())
            .file_name(format!("{}.png", device_asset_id))
            .mime_str("image/png")
            .map_err(|e| Error::ImmichApi(format!("Mime: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .text("deviceAssetId", device_asset_id.to_string())
            .text("deviceId", "koram".to_string())
            .text("fileCreatedAt", file_created_at.to_string())
            .text("fileModifiedAt", file_created_at.to_string())
            .part("assetData", part);

        let resp = self.client.post(&url)
            .header("x-api-key", &self.api_key)
            .multipart(form).send().await?;
        let resp = Self::check(resp, "Upload asset").await?;
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

    fn demo_client() -> ImmichClient {
        ImmichClient::new(&ApiConfig {
            api_key: "1bpgd3LpG30Zr3IEPNV3sWhIEqMuUGmzK3jWNh59JU".into(),
            base_url: "https://demo.immich.app/api".into(),
            public_url: None,
            timeout_secs: 30,
        }).unwrap()
    }

    #[tokio::test]
    #[ignore]
    async fn demo_get_people() {
        let people = demo_client().get_people().await.unwrap();
        assert!(!people.is_empty(), "demo server returned no people");
    }
}
