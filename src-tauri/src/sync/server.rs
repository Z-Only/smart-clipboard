use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use super::protocol::SyncMessage;
use super::SyncManager;

pub fn spawn(sync_manager: Arc<SyncManager>) {
    tauri::async_runtime::spawn(async move {
        let addr = format!("0.0.0.0:{}", sync_manager.get_config().port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(listener) => {
                info!("Sync WebSocket server listening on {addr}");
                listener
            }
            Err(err) => {
                error!("Failed to bind sync WebSocket server on {addr}: {err}");
                return;
            }
        };

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let manager = sync_manager.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(err) = handle_connection(manager, stream).await {
                            warn!("Incoming sync connection {peer_addr} closed with error: {err}");
                        }
                    });
                }
                Err(err) => warn!("Failed to accept sync connection: {err}"),
            }
        }
    });
}

async fn handle_connection(
    sync_manager: Arc<SyncManager>,
    stream: tokio::net::TcpStream,
) -> Result<(), String> {
    let ws_stream = accept_async(stream).await.map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws_stream.split();

    let hello = match read.next().await {
        Some(Ok(Message::Text(text))) => SyncMessage::from_text(text.as_str())?,
        Some(Ok(_)) => return Err("First WebSocket frame was not text hello".to_string()),
        Some(Err(err)) => return Err(err.to_string()),
        None => return Err("Connection closed before hello".to_string()),
    };

    let (remote_device_id, remote_device_name, remote_port) = match hello {
        SyncMessage::Hello {
            device_id,
            device_name,
            port,
            ..
        } => (device_id, device_name, port),
        _ => return Err("First protocol message must be Hello".to_string()),
    };

    if !sync_manager.accept_incoming_connection(&remote_device_id) {
        let response = SyncMessage::HelloAck {
            device_id: sync_manager.local_device_info().device_id,
            accepted: false,
            reason: Some("Device is not paired or sync is disabled".to_string()),
            protocol_version: 1,
        };
        write
            .send(Message::Text(response.to_text()?.into()))
            .await
            .map_err(|e| e.to_string())?;
        return Err(format!("Rejected unpaired device {}", remote_device_id));
    }

    sync_manager.handle_hello(
        &remote_device_id,
        Some(remote_device_name.clone()),
        Some(remote_port),
    );
    sync_manager.mark_connected(&remote_device_id, Some("server".to_string()));
    sync_manager.touch_last_sync(format!(
        "Accepted WebSocket transport connection from {}.",
        remote_device_name
    ));

    let response = SyncMessage::HelloAck {
        device_id: sync_manager.local_device_info().device_id,
        accepted: true,
        reason: None,
        protocol_version: 1,
    };
    write
        .send(Message::Text(response.to_text()?.into()))
        .await
        .map_err(|e| e.to_string())?;

    while let Some(message) = read.next().await {
        match message.map_err(|e| e.to_string())? {
            Message::Text(text) => match SyncMessage::from_text(text.as_str())? {
                SyncMessage::Ping { ts } => {
                    sync_manager.mark_ping(&remote_device_id);
                    let pong = SyncMessage::Pong { ts };
                    write
                        .send(Message::Text(pong.to_text()?.into()))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                SyncMessage::Pong { .. } => {
                    sync_manager.mark_pong(&remote_device_id);
                }
                SyncMessage::Disconnect { reason } => {
                    sync_manager.mark_disconnected(&remote_device_id, Some(reason));
                    break;
                }
                SyncMessage::ClipboardSyncPlaceholder { entry_hash, .. } => {
                    let ack = SyncMessage::SyncAck {
                        entry_hash,
                        accepted: true,
                    };
                    write
                        .send(Message::Text(ack.to_text()?.into()))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                SyncMessage::SyncAck { .. }
                | SyncMessage::Hello { .. }
                | SyncMessage::HelloAck { .. } => {}
            },
            Message::Close(_) => break,
            Message::Ping(payload) => {
                write
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Message::Pong(_) => sync_manager.mark_pong(&remote_device_id),
            _ => {}
        }
    }

    sync_manager.mark_disconnected(
        &remote_device_id,
        Some("Incoming socket closed".to_string()),
    );
    Ok(())
}
