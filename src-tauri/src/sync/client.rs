use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use log::{debug, info, warn};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::protocol::SyncMessage;
use super::{heartbeat_interval, reconnect_backoff, SyncManager};

pub fn spawn(sync_manager: Arc<SyncManager>) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            ticker.tick().await;
            if let Err(err) = reconcile_clients(sync_manager.clone()).await {
                warn!("Sync client reconcile error: {err}");
            }
        }
    });
}

async fn reconcile_clients(sync_manager: Arc<SyncManager>) -> Result<(), String> {
    if !sync_manager.is_sync_enabled() {
        return Ok(());
    }

    let paired = sync_manager.get_paired_devices()?;
    for device in paired.into_iter().filter(|d| d.is_active) {
        let should_attempt = matches!(
            device.status.as_str(),
            "online" | "offline" | "connecting" | "reconnecting" | "error"
        );
        if !should_attempt {
            continue;
        }

        let manager = sync_manager.clone();
        tauri::async_runtime::spawn(async move {
            let _ = connect_device_loop(manager, device).await;
        });
    }

    Ok(())
}

async fn connect_device_loop(
    sync_manager: Arc<SyncManager>,
    device: crate::storage::PairedDevice,
) -> Result<(), String> {
    let local = sync_manager.local_device_info();
    let url = format!("ws://{}:{}", device.host, device.port);
    let mut attempt = 0usize;

    loop {
        if !sync_manager.is_sync_enabled() || !device.is_active {
            sync_manager.mark_disconnected(&device.id, Some("Sync disabled".to_string()));
            return Ok(());
        }

        sync_manager.mark_connecting(&device.id, Some("client".to_string()));
        info!("Connecting to paired device {} via {}", device.id, url);

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                sync_manager.mark_connected(&device.id, Some("client".to_string()));
                sync_manager.touch_last_sync(format!(
                    "Connected to paired device {} over WebSocket.",
                    device.name
                ));

                let (mut write, mut read) = ws_stream.split();
                let pair_request = SyncMessage::PairRequest {
                    device_id: local.device_id.clone(),
                    device_name: local.device_name.clone(),
                    protocol_version: 1,
                    port: local.port,
                    public_key: sync_manager.local_public_key_base64(),
                };
                write
                    .send(Message::Text(pair_request.to_text()?.into()))
                    .await
                    .map_err(|e| e.to_string())?;

                match read.next().await {
                    Some(Ok(Message::Text(text))) => match SyncMessage::from_text(text.as_str())? {
                        SyncMessage::PairResponse {
                            accepted,
                            reason,
                            public_key,
                            ..
                        } => {
                            if !accepted {
                                sync_manager.mark_error(
                                    &device.id,
                                    reason.unwrap_or_else(|| "Pair request rejected".to_string()),
                                );
                                return Ok(());
                            }
                            let public_key = public_key
                                .ok_or_else(|| "Pair response missing public key".to_string())?;
                            sync_manager.ensure_pairing_secret(
                                &device.id,
                                Some(device.name.clone()),
                                Some(device.host.clone()),
                                Some(device.port as u16),
                                &public_key,
                            )?;
                        }
                        SyncMessage::HelloAck {
                            accepted, reason, ..
                        } => {
                            if !accepted {
                                sync_manager.mark_error(
                                    &device.id,
                                    reason.unwrap_or_else(|| "Hello rejected".to_string()),
                                );
                                return Ok(());
                            }
                        }
                        other => {
                            sync_manager.mark_error(
                                &device.id,
                                format!("Unexpected handshake response: {:?}", other),
                            );
                            return Ok(());
                        }
                    },
                    Some(Ok(_)) => {
                        sync_manager
                            .mark_error(&device.id, "Handshake ack was not text".to_string());
                        return Ok(());
                    }
                    Some(Err(err)) => {
                        sync_manager.mark_error(&device.id, err.to_string());
                        return Ok(());
                    }
                    None => {
                        sync_manager.mark_error(&device.id, "Remote closed before ack".to_string());
                        return Ok(());
                    }
                }

                let hello = SyncMessage::Hello {
                    device_id: local.device_id.clone(),
                    device_name: local.device_name.clone(),
                    protocol_version: 1,
                    port: local.port,
                };
                let encrypted_hello = sync_manager.encrypt_protocol_message(&device.id, &hello)?;
                write
                    .send(Message::Text(encrypted_hello.to_text()?.into()))
                    .await
                    .map_err(|e| e.to_string())?;

                let initial_payload = sync_manager.build_encrypted_placeholder(&device.id)?;
                write
                    .send(Message::Text(initial_payload.to_text()?.into()))
                    .await
                    .map_err(|e| e.to_string())?;

                let mut heartbeat = tokio::time::interval(heartbeat_interval());
                loop {
                    tokio::select! {
                        _ = heartbeat.tick() => {
                            let ts = chrono::Local::now().timestamp_millis();
                            sync_manager.mark_ping(&device.id);
                            write.send(Message::Text(SyncMessage::Ping { ts }.to_text()?.into())).await.map_err(|e| e.to_string())?;
                            debug!("Sent ping to {}", device.id);
                        }
                        maybe_msg = read.next() => {
                            match maybe_msg {
                                Some(Ok(Message::Text(text))) => match sync_manager.decrypt_protocol_message(&device.id, SyncMessage::from_text(text.as_str())?)? {
                                    SyncMessage::Ping { ts } => {
                                        let encrypted = sync_manager.encrypt_protocol_message(&device.id, &SyncMessage::Pong { ts })?;
                                        write.send(Message::Text(encrypted.to_text()?.into())).await.map_err(|e| e.to_string())?;
                                    }
                                    SyncMessage::Pong { .. } => sync_manager.mark_pong(&device.id),
                                    SyncMessage::Disconnect { reason } => {
                                        sync_manager.mark_disconnected(&device.id, Some(reason));
                                        break;
                                    }
                                    SyncMessage::ClipboardSyncPlaceholder { entry_hash, .. } => {
                                        let ack = sync_manager.encrypt_protocol_message(&device.id, &SyncMessage::SyncAck { entry_hash, accepted: true })?;
                                        write.send(Message::Text(ack.to_text()?.into())).await.map_err(|e| e.to_string())?;
                                    }
                                    SyncMessage::KeyVerification { fingerprint, device_id, verified } => {
                                        log::info!("Received key verification from {device_id}: fingerprint={fingerprint}, verified={verified}");
                                    }
                                    SyncMessage::SyncAck { .. } | SyncMessage::Hello { .. } | SyncMessage::HelloAck { .. } | SyncMessage::PairRequest { .. } | SyncMessage::PairResponse { .. } | SyncMessage::EncryptedPayload { .. } | SyncMessage::ClipboardSync { .. } => {}
                                },
                                Some(Ok(Message::Ping(payload))) => {
                                    write.send(Message::Pong(payload)).await.map_err(|e| e.to_string())?;
                                }
                                Some(Ok(Message::Pong(_))) => sync_manager.mark_pong(&device.id),
                                Some(Ok(Message::Close(_))) | None => {
                                    sync_manager.mark_disconnected(&device.id, Some("Socket closed".to_string()));
                                    break;
                                }
                                Some(Ok(_)) => {}
                                Some(Err(err)) => {
                                    sync_manager.mark_disconnected(&device.id, Some(err.to_string()));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                let delay = reconnect_backoff(attempt);
                sync_manager.mark_reconnect_scheduled(
                    &device.id,
                    delay.as_secs(),
                    Some(err.to_string()),
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }
        }

        let delay = reconnect_backoff(attempt);
        sync_manager.mark_reconnect_scheduled(
            &device.id,
            delay.as_secs(),
            Some("Connection dropped".to_string()),
        );
        tokio::time::sleep(delay).await;
        attempt += 1;
    }
}