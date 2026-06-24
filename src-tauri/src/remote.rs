use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::library::Library;

#[derive(Clone)]
pub struct RemoteState {
    pub library: Arc<Mutex<Library>>,
    pub app_handle: AppHandle,
}

pub async fn start_server(state: RemoteState) {
    let app = Router::new()
        .route("/", get(index_html))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("[REMOTE] Servidor rodando em http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn index_html() -> Html<&'static str> {
    Html(include_str!("../remote_client.html"))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<RemoteState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "search")]
    Search { query: String },
    #[serde(rename = "enqueue")]
    Enqueue { song_id: i64, singer: String },
    #[serde(rename = "get_queue")]
    GetQueue,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "search_results")]
    SearchResults { results: Vec<crate::library::SongEntry> },
    #[serde(rename = "queue_state")]
    QueueState { queue: Vec<crate::library::QueueEntry> },
    #[serde(rename = "notification")]
    Notification { message: String },
}

async fn handle_socket(mut socket: WebSocket, state: RemoteState) {
    // Enviar o estado inicial da fila
    {
        let lib = state.library.lock().await;
        if let Ok(queue) = lib.get_queue().await {
            let msg = ServerMessage::QueueState { queue };
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = socket.send(Message::Text(json)).await;
            }
        }
    }

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Search { query } => {
                        let lib = state.library.lock().await;
                        if let Ok(results) = lib.search_songs(&query, "title").await {
                            let response = ServerMessage::SearchResults { results };
                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = socket.send(Message::Text(json)).await;
                            }
                        }
                    }
                    ClientMessage::Enqueue { song_id, singer } => {
                        let lib = state.library.lock().await;
                        if let Ok(_) = lib.enqueue(song_id, &singer).await {
                            // Notifica o frontend principal do Tauri para atualizar a fila
                            let _ = state.app_handle.emit_all("queue_updated", ());
                            
                            // Envia notificação de sucesso para o celular
                            let response = ServerMessage::Notification { message: format!("Música adicionada para {}!", singer) };
                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = socket.send(Message::Text(json)).await;
                            }
                            
                            // Atualiza a fila neste cliente
                            if let Ok(queue) = lib.get_queue().await {
                                let msg = ServerMessage::QueueState { queue };
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    let _ = socket.send(Message::Text(json)).await;
                                }
                            }
                        }
                    }
                    ClientMessage::GetQueue => {
                        let lib = state.library.lock().await;
                        if let Ok(queue) = lib.get_queue().await {
                            let msg = ServerMessage::QueueState { queue };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                let _ = socket.send(Message::Text(json)).await;
                            }
                        }
                    }
                }
            }
        }
    }
}
