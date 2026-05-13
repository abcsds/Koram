use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CONFIG_PATH: &str = "config/koram.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_key: String,
    pub base_url: String,
    /// User-facing Immich URL (e.g. `https://photos.example.com`). Used to build
    /// browser-clickable deep links to /people/<id>. Falls back to `base_url`
    /// (minus `/api`) when unset — fine for a single-host setup, required when
    /// `base_url` points at an internal address (e.g. `http://immich-server:2283`).
    #[serde(default)]
    pub public_url: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 { 30 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api: ApiConfig,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    #[serde(default = "default_album_name")]
    pub album_name: String,
}

fn default_cache_dir() -> PathBuf { PathBuf::from("cache") }
fn default_album_name() -> String { "Koram Graphs".into() }

impl Config {
    pub fn from_env() -> Self {
        let public_url = std::env::var("IMMICH_PUBLIC_URL").ok().filter(|s| !s.is_empty());
        Self {
            api: ApiConfig {
                api_key: std::env::var("IMMICH_API_KEY").unwrap_or_default(),
                base_url: std::env::var("IMMICH_BASE_URL").unwrap_or_default(),
                public_url,
                timeout_secs: 30,
            },
            cache_dir: default_cache_dir(),
            album_name: default_album_name(),
        }
    }

    pub fn from_file(path: &std::path::Path) -> crate::error::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        toml::from_str(&s).map_err(|e| crate::error::Error::Config(e.to_string()))
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> crate::error::Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let s = toml::to_string_pretty(self).map_err(|e| crate::error::Error::Config(e.to_string()))?;
        std::fs::write(path, s)?;
        Ok(())
    }

    pub fn with_env(mut self) -> Self {
        if let Ok(v) = std::env::var("IMMICH_API_KEY") { if !v.is_empty() { self.api.api_key = v; } }
        if let Ok(v) = std::env::var("IMMICH_BASE_URL") { if !v.is_empty() { self.api.base_url = v; } }
        if let Ok(v) = std::env::var("IMMICH_PUBLIC_URL") { if !v.is_empty() { self.api.public_url = Some(v); } }
        self
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        if self.api.api_key.is_empty() {
            return Err(crate::error::Error::Config("IMMICH_API_KEY not set".into()));
        }
        if self.api.base_url.is_empty() {
            return Err(crate::error::Error::Config("IMMICH_BASE_URL not set".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_key() {
        let cfg = Config {
            api: ApiConfig { api_key: "".into(), base_url: "http://x".into(), public_url: None, timeout_secs: 30 },
            cache_dir: "cache".into(),
            album_name: "K".into(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn roundtrip_toml() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.toml");
        let cfg = Config {
            api: ApiConfig { api_key: "abc".into(), base_url: "http://x".into(), public_url: None, timeout_secs: 30 },
            cache_dir: "cache".into(),
            album_name: "Koram Graphs".into(),
        };
        cfg.save_to_file(&p).unwrap();
        let loaded = Config::from_file(&p).unwrap();
        assert_eq!(loaded.api.api_key, "abc");
        assert_eq!(loaded.album_name, "Koram Graphs");
    }
}
