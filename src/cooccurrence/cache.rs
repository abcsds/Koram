use crate::error::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn cache_key(person_ids: &[String], from: Option<&str>, to: Option<&str>) -> String {
    let mut sorted = person_ids.to_vec();
    sorted.sort();
    let mut hasher = Sha256::new();
    hasher.update(sorted.join(",").as_bytes());
    hasher.update(b"|");
    hasher.update(from.unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(to.unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

pub fn cache_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_dir.join(format!("{}.json", key))
}

pub fn read<T: DeserializeOwned>(cache_dir: &Path, key: &str) -> Result<Option<T>> {
    let p = cache_path(cache_dir, key);
    if !p.exists() { return Ok(None); }
    let s = std::fs::read_to_string(&p)?;
    Ok(Some(serde_json::from_str(&s)?))
}

pub fn write<T: Serialize>(cache_dir: &Path, key: &str, value: &T) -> Result<()> {
    std::fs::create_dir_all(cache_dir)?;
    let p = cache_path(cache_dir, key);
    let tmp = p.with_extension("json.tmp");
    let s = serde_json::to_string(value)?;
    std::fs::write(&tmp, s)?;
    std::fs::rename(&tmp, &p).map_err(Error::from)?;
    Ok(())
}

pub fn delete(cache_dir: &Path, key: &str) -> Result<()> {
    let p = cache_path(cache_dir, key);
    if p.exists() { std::fs::remove_file(p)?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_order_independent() {
        let a = cache_key(&["1".into(), "2".into(), "3".into()], None, None);
        let b = cache_key(&["3".into(), "1".into(), "2".into()], None, None);
        assert_eq!(a, b);
    }

    #[test]
    fn key_changes_with_dates() {
        let ids = vec!["a".into()];
        let a = cache_key(&ids, None, None);
        let b = cache_key(&ids, Some("2020-01-01"), None);
        let c = cache_key(&ids, None, Some("2024-12-31"));
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
    }

    #[test]
    fn roundtrip_write_read() {
        let dir = tempfile::tempdir().unwrap();
        let key = "test";
        let val = vec![("x".to_string(), 1u32), ("y".into(), 2)];
        write(dir.path(), key, &val).unwrap();
        let loaded: Option<Vec<(String, u32)>> = read(dir.path(), key).unwrap();
        assert_eq!(loaded.unwrap(), val);
    }

    #[test]
    fn read_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let loaded: Option<u32> = read(dir.path(), "nope").unwrap();
        assert_eq!(loaded, None);
    }
}
