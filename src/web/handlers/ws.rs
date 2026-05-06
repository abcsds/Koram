use crate::web::state::AppState;
use axum::{
    extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade},
    response::IntoResponse,
};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.progress_tx.subscribe();
    {
        let p = state.progress.read().await;
        if let Ok(s) = serde_json::to_string(&*p) {
            let _ = socket.send(Message::Text(s.into())).await;
        }
    }
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(p) => {
                        if let Ok(s) = serde_json::to_string(&p) {
                            if socket.send(Message::Text(s.into())).await.is_err() { break; }
                        }
                    }
                    Err(_) => break,
                }
            }
            inc = socket.recv() => {
                match inc {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}
