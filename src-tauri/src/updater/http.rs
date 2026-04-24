use futures_util::StreamExt;

pub async fn fetch_text(url: &str) -> Result<String, String> {
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {} for {}", response.status(), url));
    }
    response.text().await.map_err(|e| e.to_string())
}

pub async fn fetch_bytes_with_progress<F>(url: &str, mut on_progress: F) -> Result<Vec<u8>, String>
where
    F: FnMut(f64),
{
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {} for {}", response.status(), url));
    }

    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut bytes = Vec::new();

    on_progress(0.0);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        if let Some(total) = total {
            if total > 0 {
                on_progress((downloaded as f64 / total as f64).clamp(0.0, 1.0));
            }
        }
    }
    if total.is_none() {
        on_progress(1.0);
    }
    Ok(bytes)
}
