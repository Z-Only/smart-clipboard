use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use tauri::Emitter;
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

    let pair_request = match read.next().await {
        Some(Ok(Message::Text(text))) => SyncMessage::from_text(text.as_str())?,
        Some(Ok(_)) => return Err("First WebSocket frame was not text pair request".to_string()),
        Some(Err(err)) => return Err(err.to_string()),
        None => return Err("Connection closed before pair request".to_string()),
    };

    let (remote_device_id, remote_device_name, remote_port, remote_public_key) = match pair_request
    {
        SyncMessage::PairRequest {
            device_id,
            device_name,
            port,
            public_key,
            ..
        } => (device_id, device_name, port, public_key),
        _ => return Err("First protocol message must be PairRequest".to_string()),
    };

    if !sync_manager.is_sync_enabled() {
        let response = SyncMessage::PairResponse {
            device_id: sync_manager.local_device_info().device_id,
            device_name: sync_manager.local_device_info().device_name,
            accepted: false,
            reason: Some("Sync is disabled".to_string()),
            protocol_version: 1,
            port: sync_manager.local_device_info().port,
            public_key: None,
        };
        write
            .send(Message::Text(response.to_text()?.into()))
            .await
            .map_err(|e| e.to_string())?;
        return Err(format!(
            "Rejected device {} while sync disabled",
            remote_device_id
        ));
    }

    sync_manager.ensure_pairing_secret(
        &remote_device_id,
        Some(remote_device_name.clone()),
        Some("127.0.0.1".to_string()),
        Some(remote_port),
        &remote_public_key,
    )?;
    sync_manager.mark_connected(&remote_device_id, Some("server".to_string()));
    sync_manager.touch_last_sync(format!(
        "Accepted WebSocket transport connection from {}.",
        remote_device_name
    ));

    let response = SyncMessage::PairResponse {
        device_id: sync_manager.local_device_info().device_id,
        device_name: sync_manager.local_device_info().device_name,
        accepted: true,
        reason: None,
        protocol_version: 1,
        port: sync_manager.local_device_info().port,
        public_key: Some(sync_manager.local_public_key_base64()),
    };
    write
        .send(Message::Text(response.to_text()?.into()))
        .await
        .map_err(|e| e.to_string())?;

    let mut outgoing_rx = sync_manager.subscribe_outgoing();

    loop {
        tokio::select! {
            maybe_msg = read.next() => {
                match maybe_msg {
                    Some(Ok(Message::Text(text))) => {
                        match sync_manager.decrypt_protocol_message(&remote_device_id, SyncMessage::from_text(text.as_str())?)? {
                            SyncMessage::Ping { ts } => {
                                sync_manager.mark_ping(&remote_device_id);
                                let pong = sync_manager.encrypt_protocol_message(&remote_device_id, &SyncMessage::Pong { ts })?;
                                write.send(Message::Text(pong.to_text()?.into())).await.map_err(|e| e.to_string())?;
                            }
                            SyncMessage::Pong { .. } => {
                                sync_manager.mark_pong(&remote_device_id);
                            }
                            SyncMessage::Disconnect { reason } => {
                                sync_manager.mark_disconnected(&remote_device_id, Some(reason));
                                break;
                            }
                            SyncMessage::ClipboardSyncPlaceholder { entry_hash, .. } => {
                                let ack = sync_manager.encrypt_protocol_message(
                                    &remote_device_id,
                                    &SyncMessage::SyncAck { entry_hash, accepted: true },
                                )?;
                                write.send(Message::Text(ack.to_text()?.into())).await.map_err(|e| e.to_string())?;
                            }
                            SyncMessage::ClipboardSync { entry, sender_device_id, .. } => {
                                let accepted = match sync_manager.handle_incoming_sync(&sender_device_id, &entry) {
                                    Ok(Some(stored_entry)) => {
                                        if let Some(handle) = sync_manager.app_handle() {
                                            let _ = handle.emit("clipboard-changed", &stored_entry);
                                        }
                                        true
                                    }
                                    Ok(None) => true,
                                    Err(e) => {
                                        log::error!("Failed to handle incoming sync: {}", e);
                                        false
                                    }
                                };
                                let ack = sync_manager.encrypt_protocol_message(
                                    &remote_device_id,
                                    &SyncMessage::SyncAck { entry_hash: entry.hash.clone(), accepted },
                                )?;
                                write.send(Message::Text(ack.to_text()?.into())).await.map_err(|e| e.to_string())?;
                            }
                            SyncMessage::KeyVerification { fingerprint, device_id, verified } => {
                                log::info!("Received key verification from {device_id}: fingerprint={fingerprint}, verified={verified}");
                            }
                            SyncMessage::SyncAck { .. }
                            | SyncMessage::Hello { .. }
                            | SyncMessage::HelloAck { .. }
                            | SyncMessage::PairRequest { .. }
                            | SyncMessage::PairResponse { .. }
                            | SyncMessage::EncryptedPayload { .. } => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        write.send(Message::Pong(payload)).await.map_err(|e| e.to_string())?;
                    }
                    Some(Ok(Message::Pong(_))) => sync_manager.mark_pong(&remote_device_id),
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        log::warn!("WebSocket read error: {}", e);
                        break;
                    }
                }
            }
            result = outgoing_rx.recv() => {
                match result {
                    Ok(payload) => {
                        let entry_hash = payload.hash.clone();
                        if sync_manager.db_ref().has_sync_log(&entry_hash, &remote_device_id, "sent").unwrap_or(true) {
                            continue;
                        }
                        let sync_msg = SyncMessage::ClipboardSync {
                            entry: payload,
                            sender_device_id: sync_manager.local_device_info().device_id.clone(),
                            timestamp: chrono::Local::now().timestamp_millis(),
                        };
                        match sync_manager.encrypt_protocol_message(&remote_device_id, &sync_msg) {
                            Ok(encrypted) => {
                                if let Err(e) = write.send(Message::Text(encrypted.to_text().unwrap_or_default().into())).await {
                                    log::error!("Failed to send sync entry to {}: {}", remote_device_id, e);
                                    break;
                                }
                                let _ = sync_manager.db_ref().insert_sync_log(&entry_hash, &remote_device_id, "sent");
                                sync_manager.touch_last_sync(format!("Sent clipboard entry to {}", remote_device_id));
                            }
                            Err(e) => log::error!("Failed to encrypt sync entry for {}: {}", remote_device_id, e),
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("Outgoing broadcast lagged by {} entries for server connection {}", n, remote_device_id);
                    }
                    Err(_) => break,
                }
            }
        }
    }

    sync_manager.mark_disconnected(
        &remote_device_id,
        Some("Incoming socket closed".to_string()),
    );
    Ok(())
}
