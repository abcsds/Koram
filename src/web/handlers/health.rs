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
    pub error: Option<String>,
}

pub async fn check_connection(State(state): State<AppState>) -> Json<ConnectionStatus> {
    let cfg = state.config.read().await;
    let client = match ImmichClient::new(&cfg.api) {
        Ok(c) => c,
        Err(e) => return Json(ConnectionStatus { ok: false, immich_version: None, error: Some(e.to_string()) }),
    };
    drop(cfg);
    match client.validate_connection().await {
        Ok(info) => Json(ConnectionStatus { ok: true, immich_version: Some(info.version), error: None }),
        Err(e)   => Json(ConnectionStatus { ok: false, immich_version: None, error: Some(e.to_string()) }),
    }
}

// `StatusCode` is referenced for future use; a `_` import keeps the analyzer quiet without unused-warnings.
#[allow(dead_code)]
fn _doc_status() -> StatusCode { StatusCode::OK }
