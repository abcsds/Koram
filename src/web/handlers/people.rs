use crate::immich_api::ImmichClient;
use crate::web::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Json, Response},
};
use futures_util::stream::{self, StreamExt};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct PersonInfo {
    pub id: String,
    pub name: Option<String>,
    pub total: u64,
}

pub async fn get_people(State(state): State<AppState>)
    -> Result<Json<Vec<PersonInfo>>, (StatusCode, String)>
{
    let cfg = state.config.read().await;
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);
    let people = client.get_people().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Per-person asset count comes from /people/{id}/statistics. Immich has no batch
    // endpoint, so fan out with a bounded concurrency to avoid hammering the server.
    // Failures fall through as 0 — a stale 0 is preferable to failing the whole list.
    let ids: Vec<String> = people.iter().map(|p| p.id.clone()).collect();
    let counts: HashMap<String, u64> = stream::iter(ids)
        .map(|id| {
            let client = client.clone();
            async move {
                let n = match client.get_person_statistics(&id).await {
                    Ok(n) => n,
                    Err(e) => {
                        tracing::warn!("statistics fetch failed for {}: {}", id, e);
                        0
                    }
                };
                (id, n)
            }
        })
        .buffer_unordered(16)
        .collect()
        .await;

    let out: Vec<PersonInfo> = people.into_iter().map(|p| {
        let total = counts.get(&p.id).copied().unwrap_or(0);
        PersonInfo { id: p.id, name: p.name, total }
    }).collect();
    Ok(Json(out))
}

pub async fn get_person_thumbnail(
    State(state): State<AppState>,
    Path(person_id): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let cfg = state.config.read().await;
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);
    let (bytes, ct) = client.get_person_thumbnail(&person_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, ct)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
