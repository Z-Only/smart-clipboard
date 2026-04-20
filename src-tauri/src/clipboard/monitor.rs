use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use super::types::{ClipboardChange, ImageData};

pub struct ClipboardMonitor {
    interval: Duration,
    running: Arc<AtomicBool>,
}

impl ClipboardMonitor {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval: Duration::from_millis(interval_ms),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self, tx: mpsc::UnboundedSender<ClipboardChange>) {
        let running = self.running.clone();
        let interval = self.interval;

        running.store(true, Ordering::SeqCst);

        // arboard::Clipboard is !Send, so we must create and use it within a dedicated thread
        std::thread::spawn(move || {
            let mut clipboard = match arboard::Clipboard::new() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to init clipboard: {}", e);
                    return;
                }
            };

            let mut last_hash: Option<String> = None;

            while running.load(Ordering::SeqCst) {
                // Check for text first
                if let Ok(text) = clipboard.get_text() {
                    if !text.trim().is_empty() {
                        let hash = format!("{:x}", Sha256::digest(text.as_bytes()));

                        let is_new = match &last_hash {
                            Some(prev) => prev != &hash,
                            None => true,
                        };

                        if is_new {
                            last_hash = Some(hash);
                            let text_change = ClipboardChange {
                                content: text,
                                content_type: "text".to_string(),
                                source_app: None,
                                image_data: None,
                            };
                            if tx.send(text_change).is_err() {
                                break;
                            }
                            std::thread::sleep(interval);
                            continue;
                        }
                    }
                }

                // Check for image if no new text was found
                if let Ok(img) = clipboard.get_image() {
                    let raw_bytes: Vec<u8> = img.bytes.into_owned();
                    let hash = format!("{:x}", Sha256::digest(&raw_bytes));

                    let is_new = match &last_hash {
                        Some(prev) => prev != &hash,
                        None => true,
                    };

                    if is_new {
                        last_hash = Some(hash);
                        let change = ClipboardChange {
                            content: String::new(),
                            content_type: "image".to_string(),
                            source_app: None,
                            image_data: Some(ImageData {
                                bytes: raw_bytes,
                                width: img.width,
                                height: img.height,
                            }),
                        };
                        if tx.send(change).is_err() {
                            break;
                        }
                    }
                }

                std::thread::sleep(interval);
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
