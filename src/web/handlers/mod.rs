mod config;
mod graph;
mod health;
mod people;
mod upload;
mod ws;

use crate::web::state::AppState;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tower_http::services::ServeDir;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health",                  get(health::health_check))
        .route("/api/connection",              get(health::check_connection))
        .route("/api/people",                  get(people::get_people))
        .route("/api/people/{person_id}/thumbnail", get(people::get_person_thumbnail))
        .route("/api/graph/compute",           axum::routing::post(graph::compute_graph))
        .route("/api/graph/result",            get(graph::get_result))
        .route("/api/graph/cancel",            axum::routing::post(graph::cancel_graph))
        .route("/api/ws",                      get(ws::ws_handler))
        .route("/api/upload",                  axum::routing::post(upload::upload_to_immich))
        .route("/api/config",
            get(config::get_config).put(config::update_config))
        .route("/api/config/defaults",         get(config::get_config_defaults))
        // Serve built frontend, falling back to index.html so SPA deep links work.
        .fallback_service(ServeDir::new("frontend/dist").fallback(get(serve_index)))
        .with_state(state)
}

async fn serve_index() -> impl IntoResponse {
    match tokio::fs::read_to_string("frontend/dist/index.html").await {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(
            r#"<!DOCTYPE html><html><body style="font-family:sans-serif;background:#050506;color:#EDEDEF;display:grid;place-items:center;height:100vh;margin:0">
<div style="text-align:center"><h1>Frontend not built</h1>
<p>Run <code>cd frontend &amp;&amp; npm install &amp;&amp; npm run build</code></p></div></body></html>"#,
        ).into_response(),
    }
}
