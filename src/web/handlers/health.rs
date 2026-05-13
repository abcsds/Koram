use crate::immich_api::ImmichClient;
use crate::web::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse { pub ok: bool }

pub async fn health_check() -> Json<HealthResponse> { Json(HealthResponse { ok: true }) }

#[derive(Serialize)]
pub struct ConnectionStatus {
    pub ok: bool,
    pub immich_version: Option<String>,
    pub immich_base_url: Option<String>,
    pub error: Option<String>,
}

pub async fn check_connection(State(state): State<AppState>) -> Json<ConnectionStatus> {
    let cfg = state.config.read().await;
    // The frontend uses this to deep-link into Immich (e.g. /people/{id}). Prefer
    // an explicit IMMICH_PUBLIC_URL when set — IMMICH_BASE_URL is often an internal
    // container address (e.g. http://immich-server:2283) that won't resolve in a
    // user's browser. Strip a trailing `/api` either way.
    let immich_base_url = {
        let raw = cfg.api.public_url.as_deref().unwrap_or(&cfg.api.base_url);
        let raw = raw.trim_end_matches('/');
        let trimmed = raw.strip_suffix("/api").unwrap_or(raw);
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    };
    let client = match ImmichClient::new(&cfg.api) {
        Ok(c) => c,
        Err(e) => return Json(ConnectionStatus {
            ok: false, immich_version: None, immich_base_url, error: Some(e.to_string()),
        }),
    };
    drop(cfg);
    match client.validate_connection().await {
        Ok(info) => Json(ConnectionStatus {
            ok: true, immich_version: Some(info.version), immich_base_url, error: None,
        }),
        Err(e)   => Json(ConnectionStatus {
            ok: false, immich_version: None, immich_base_url, error: Some(e.to_string()),
        }),
    }
}

// `StatusCode` is referenced for future use; a `_` import keeps the analyzer quiet without unused-warnings.
#[allow(dead_code)]
fn _doc_status() -> StatusCode { StatusCode::OK }
