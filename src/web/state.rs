use crate::config::Config;
use crate::cooccurrence::CoOccurrenceResult;
use crate::job::Progress;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub progress: Arc<RwLock<Progress>>,
    pub progress_tx: broadcast::Sender<Progress>,
    pub cancel_token: Arc<RwLock<Option<CancellationToken>>>,
    pub last_result: Arc<RwLock<Option<CoOccurrenceResult>>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            config: Arc::new(RwLock::new(config)),
            progress: Arc::new(RwLock::new(Progress::default())),
            progress_tx: tx,
            cancel_token: Arc::new(RwLock::new(None)),
            last_result: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn create_cancel_token(&self) -> CancellationToken {
        let token = CancellationToken::new();
        *self.cancel_token.write().await = Some(token.clone());
        token
    }

    pub async fn request_cancel(&self) -> bool {
        if let Some(t) = self.cancel_token.read().await.as_ref() {
            t.cancel();
            true
        } else {
            false
        }
    }
}
