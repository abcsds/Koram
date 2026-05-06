use crate::config::Config;
use crate::web::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};

pub async fn get_config(State(state): State<AppState>) -> Json<Config> {
    Json(state.config.read().await.clone())
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(new): Json<Config>,
) -> Result<Json<Config>, (StatusCode, String)> {
    *state.config.write().await = new.clone();
    new.save_to_file(std::path::Path::new(crate::config::CONFIG_PATH))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(new))
}

pub async fn get_config_defaults() -> Json<Config> {
    // A blank-env config gives us the default `cache_dir` and `album_name`,
    // with API fields empty so the UI can show defaults next to the user's overrides.
    Json(Config::from_env())
}
