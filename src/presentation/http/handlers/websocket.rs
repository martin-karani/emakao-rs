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
    domain::enums::SenderType,
    presentation::app_state::AppState,
};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = match params.get("token") {
        Some(t) => t.clone(),
        None => return AppError::Unauthorised.into_response(),
    };

    // Validate token via identity sub-struct
    let claims = match state.identity.auth_port.verify_token(&token) {
        Ok(c) => c,
        Err(_) => return AppError::Unauthorised.into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, claims))
}

async fn handle_socket(ws: WebSocket, state: AppState, claims: crate::domain::auth::JwtClaims) {
    let (mut sender, mut receiver) = ws.split();

    // mpsc channel to funnel outgoing messages from multiple tasks into one sender
    let (tx_ws, mut rx_ws) = tokio::sync::mpsc::channel::<Message>(100);

    tokio::spawn(async move {
        while let Some(msg) = rx_ws.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let agency_id = claims.agency_id;

    // Resolve tenant pool via infra sub-struct
    let tenant_pool = match state.infra.tenant_pools.for_agency(agency_id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "ws: failed to get agency pool");
            return;
        }
    };

    let msg_repo = Arc::new(
        crate::infrastructure::db::message_repository_sqlx::PgMessageRepo::from(
            tenant_pool.clone(),
        ),
    );

    let user_id = claims.sub;
    let _ = tx_ws.send(Message::Text("connected".into())).await;

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
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
                        // WebSocket channels live on runtime sub-struct
                        let channels = &state.runtime.websocket_channels;
                        let tx = channels
                            .entry(conv_id)
                            .or_insert_with(|| {
                                let (tx, _) = broadcast::channel(100);
                                tx
                            })
                            .clone();

                        let mut rx = tx.subscribe();
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
                        if let Ok(msg) = msg_repo
                            .create(crate::domain::message::CreateMessageCommand {
                                conversation_id: conv_id,
                                sender_id: user_id,
                                sender_type: SenderType::Staff,
                                body: body.to_string(),
                            })
                            .await
                        {
                            // Broadcast to all subscribers of that conversation
                            if let Some(tx) = state.runtime.websocket_channels.get(&conv_id) {
                                let notification = serde_json::json!({
                                    "type": "new_message",
                                    "message": {
                                        "id":              msg.id,
                                        "conversation_id": msg.conversation_id,
                                        "sender_id":       msg.sender_id,
                                        "sender_type":     msg.sender_type,
                                        "body":            msg.body,
                                        "created_at":      msg.created_at,
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
