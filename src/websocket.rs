use crate::models::WsMessage;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct WebSocketManager {
    sender: broadcast::Sender<WsMessage>,
}

impl WebSocketManager {
    pub fn new() -> (Self, broadcast::Sender<WsMessage>) {
        let (sender, _) = broadcast::channel(1000);
        let manager = Self {
            sender: sender.clone(),
        };
        (manager, sender)
    }

    pub fn get_sender(&self) -> broadcast::Sender<WsMessage> {
        self.sender.clone()
    }
}

/// WebSocket handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(sender): State<Arc<broadcast::Sender<WsMessage>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, sender))
}

async fn handle_socket(socket: WebSocket, sender: Arc<broadcast::Sender<WsMessage>>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = sender.subscribe();

    // Spawn a task to send messages from the broadcast channel to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn a task to receive messages from the WebSocket (for subscription management)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_receiver.next().await {
            match message {
                Message::Text(text) => {
                    log::debug!("Received WebSocket message: {}", text);
                    // Handle subscription requests here
                }
                Message::Close(_) => {
                    log::info!("WebSocket connection closed");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    log::info!("WebSocket connection terminated");
}
