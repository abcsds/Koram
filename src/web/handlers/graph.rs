use crate::cooccurrence::{cache, compute, CoOccurrenceResult};
use crate::immich_api::ImmichClient;
use crate::job::Progress;
use crate::web::state::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ComputeRequest {
    pub person_ids: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize)]
pub struct ComputeResponse {
    pub cached: bool,
    pub result: Option<CoOccurrenceResult>,
    pub key: String,
}

pub async fn compute_graph(
    State(state): State<AppState>,
    Json(req): Json<ComputeRequest>,
) -> Result<Json<ComputeResponse>, (StatusCode, String)> {
    let cfg = state.config.read().await;
    let cache_dir = cfg.cache_dir.clone();
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);

    let key = cache::cache_key(&req.person_ids, req.from.as_deref(), req.to.as_deref());

    // Cache hit short-circuits the job entirely.
    if !req.force {
        if let Some(cached) = cache::read::<CoOccurrenceResult>(&cache_dir, &key)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            *state.last_result.write().await = Some(cached.clone());
            return Ok(Json(ComputeResponse { cached: true, result: Some(cached), key }));
        }
    } else {
        let _ = cache::delete(&cache_dir, &key);
    }

    let people_meta = client.get_people().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let progress_tx = state.progress_tx.clone();
    let total = req.person_ids.len() as u32;

    // Atomic test-and-set: refuse to start if another job is already running, otherwise
    // claim the slot under the same write guard to prevent two POSTs racing past the check.
    let initial = Progress {
        status: "running".into(),
        processed: 0,
        total,
        current_person_id: None,
        current_person_name: None,
        message: None,
    };
    {
        let mut p = state.progress.write().await;
        if p.status == "running" {
            return Err((StatusCode::CONFLICT, "another compute is already running".into()));
        }
        *p = initial.clone();
    }
    let _ = progress_tx.send(initial);

    // Cancellation token created *after* the slot is claimed so a conflicting POST can't
    // overwrite a live token in the AppState.
    let cancel = state.create_cancel_token().await;

    // Spawn the compute. The HTTP request returns immediately.
    let state_bg = state.clone();
    let key_bg = key.clone();
    let cache_dir_bg = cache_dir.clone();
    let req_for_args = req;
    tokio::spawn(async move {
        let args = compute::ComputeArgs {
            client,
            selected_person_ids: req_for_args.person_ids,
            from: req_for_args.from.clone(),
            to: req_for_args.to.clone(),
            people_meta,
            cancel,
            progress_tx: state_bg.progress_tx.clone(),
        };

        let outcome = compute::compute(args).await;

        let final_progress = match &outcome {
            Ok(result) => {
                if let Err(e) = cache::write(&cache_dir_bg, &key_bg, result) {
                    tracing::warn!("Failed to persist cache: {}", e);
                }
                *state_bg.last_result.write().await = Some(result.clone());
                Progress {
                    status: "completed".into(),
                    processed: total,
                    total,
                    current_person_id: None,
                    current_person_name: None,
                    message: Some(key_bg.clone()),
                }
            }
            Err(crate::error::Error::Cancelled) => Progress {
                status: "cancelled".into(),
                processed: 0, total,
                current_person_id: None, current_person_name: None,
                message: None,
            },
            Err(e) => Progress {
                status: "error".into(),
                processed: 0, total,
                current_person_id: None, current_person_name: None,
                message: Some(e.to_string()),
            },
        };

        *state_bg.progress.write().await = final_progress.clone();
        let _ = state_bg.progress_tx.send(final_progress);
    });

    Ok(Json(ComputeResponse { cached: false, result: None, key }))
}

#[derive(Deserialize)]
pub struct ResultQuery { pub key: String }

pub async fn get_result(
    State(state): State<AppState>,
    Query(q): Query<ResultQuery>,
) -> Result<Json<CoOccurrenceResult>, (StatusCode, String)> {
    let cfg = state.config.read().await;
    let cache_dir = cfg.cache_dir.clone();
    drop(cfg);
    let cached = cache::read::<CoOccurrenceResult>(&cache_dir, &q.key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "not in cache".into()))?;
    Ok(Json(cached))
}

#[derive(Serialize)]
pub struct CancelResponse { pub cancelled: bool }

pub async fn cancel_graph(State(state): State<AppState>) -> Json<CancelResponse> {
    Json(CancelResponse { cancelled: state.request_cancel().await })
}
