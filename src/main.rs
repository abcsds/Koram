use koram::{
    config::{Config, CONFIG_PATH},
    web::{self, AppState},
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "koram=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load_config()?;
    let state = AppState::new(config);
    let app = web::create_router(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 5001));
    tracing::info!("koram listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn load_config() -> anyhow::Result<Config> {
    let p = std::path::Path::new(CONFIG_PATH);
    let cfg = if p.exists() {
        Config::from_file(p)?.with_env()
    } else {
        let default = Config::from_env();
        if let Err(e) = default.save_to_file(p) {
            tracing::warn!("Could not write default config to {}: {}", CONFIG_PATH, e);
        }
        default
    };
    if let Err(e) = cfg.validate() {
        tracing::warn!("Config incomplete: {}", e);
    }
    Ok(cfg)
}
