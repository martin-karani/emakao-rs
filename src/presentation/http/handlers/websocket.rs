use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::message_repository::MessageRepository},
    presentation::app_state::AppState,
};

type ConversationChannels = Arc<dashmap::DashMap<Uuid, broadcast::Sender<String>>>;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    // Authenticate via token from query parameter
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = match params.get("token") {
        Some(t) => t.clone(),
        None => return AppError::Unauthorised.into_response(),
    };

    // Validate token
    let claims = match state.auth_port.verify_token(&token) {
        Ok(c) => c,
        Err(_) => return AppError::Unauthorised.into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, claims))
}

async fn handle_socket(ws: WebSocket, state: AppState, claims: crate::domain::auth::JwtClaims) {
    let (mut sender, mut receiver) = ws.split();

    // Create an mpsc channel to handle outgoing messages from multiple tasks
    let (tx_ws, mut rx_ws) = tokio::sync::mpsc::channel::<Message>(100);

    // Spawn a task to forward messages from our internal channel to the websocket sender
    tokio::spawn(async move {
        while let Some(msg) = rx_ws.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Resolve agency and pool from the claims (we can look up the agency_id from the token)
    let agency_id = claims.agency_id;
    let tenant_pool = match state.tenant_pools.for_agency(agency_id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to get tenant pool");
            return;
        }
    };

    // Create repositories
    let msg_repo = Arc::new(
        crate::infrastructure::db::message_repository_sqlx::PgMessageRepo::from(
            tenant_pool.clone(),
        ),
    );
    let _conv_repo = Arc::new(
        crate::infrastructure::db::conversation_repository_sqlx::PgConversationRepo::from(
            tenant_pool.clone(),
        ),
    );

    // Hold the user ID
    let user_id = claims.sub;

    // Send a welcome message (optional)
    let _ = tx_ws.send(Message::Text("connected".into())).await;

    // Process messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            // Parse as JSON: { "type": "join"|"message", "conversation_id": "...", "body": "..." }
            let request: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => {
                    let _ = tx_ws.send(Message::Text("invalid json".into())).await;
                    continue;
                }
            };

            match request["type"].as_str() {
                Some("join") => {
                    if let Some(conv_id) = request["conversation_id"]
                        .as_str()
                        .and_then(|s| Uuid::parse_str(s).ok())
                    {
                        // Subscribe to the conversation channel
                        let channels = &state.websocket_channels;
                        let tx = channels
                            .entry(conv_id)
                            .or_insert_with(|| {
                                let (tx, _) = broadcast::channel(100);
                                tx
                            })
                            .clone();

                        let mut rx = tx.subscribe();
                        // Spawn a task to forward channel messages to this socket
                        let tx_ws_clone = tx_ws.clone();
                        tokio::spawn(async move {
                            while let Ok(msg) = rx.recv().await {
                                if tx_ws_clone.send(Message::Text(msg.into())).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                }
                Some("message") => {
                    if let (Some(conv_id), Some(body)) = (
                        request["conversation_id"]
                            .as_str()
                            .and_then(|s| Uuid::parse_str(s).ok()),
                        request["body"].as_str(),
                    ) {
                        // Save to DB
                        if let Ok(msg) = msg_repo
                            .create(crate::domain::message::CreateMessageCommand {
                                conversation_id: conv_id,
                                sender_id: user_id,
                                sender_type: "staff".into(), // could be derived from role
                                body: body.to_string(),
                            })
                            .await
                        {
                            // Broadcast to all subscribers of that conversation
                            if let Some(tx) = state.websocket_channels.get(&conv_id) {
                                let notification = serde_json::json!({
                                    "type": "new_message",
                                    "message": {
                                        "id": msg.id,
                                        "conversation_id": msg.conversation_id,
                                        "sender_id": msg.sender_id,
                                        "sender_type": msg.sender_type,
                                        "body": msg.body,
                                        "created_at": msg.created_at,
                                    }
                                })
                                .to_string();
                                let _ = tx.send(notification);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
