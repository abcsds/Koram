use crate::immich_api::ImmichClient;
use crate::web::state::AppState;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct UploadResponse {
    pub asset_id: String,
    pub album_id: String,
}

pub async fn upload_to_immich(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let mut png_bytes: Option<bytes::Bytes> = None;
    let mut device_asset_id = String::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart: {e}")))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                let b = field.bytes().await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                png_bytes = Some(b);
            }
            "deviceAssetId" => {
                device_asset_id = field.text().await.unwrap_or_default();
            }
            _ => { let _ = field.bytes().await; }
        }
    }

    let png = png_bytes.ok_or((StatusCode::BAD_REQUEST, "missing image field".into()))?;
    if device_asset_id.is_empty() {
        device_asset_id = format!("koram-{}", uuid::Uuid::new_v4());
    }

    let cfg = state.config.read().await;
    let album_name = cfg.album_name.clone();
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);

    let now = chrono::Utc::now().to_rfc3339();
    let upload = client.upload_asset(png, &device_asset_id, &now).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let album = client.ensure_album(&album_name).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    client.add_assets_to_album(&album.id, &[upload.id.clone()]).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(UploadResponse { asset_id: upload.id, album_id: album.id }))
}
