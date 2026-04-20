use std::sync::Arc;

use log::info;
use reqwest::{Client, StatusCode};

use super::rate_limiter::TokenBucketLimiter;

pub struct WebDavClient {
    http: Client,
    base_url: String,
    username: String,
    password: String,
    rate_limiter: Arc<TokenBucketLimiter>,
}

pub enum PutResult {
    Ok,
    EtagConflict,
}

impl WebDavClient {
    pub fn new(
        base_url: &str,
        username: &str,
        password: &str,
        rate_limiter: Arc<TokenBucketLimiter>,
    ) -> Result<Self, String> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self {
            http,
            base_url,
            username: username.to_string(),
            password: password.to_string(),
            rate_limiter,
        })
    }

    fn url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, path)
    }

    pub async fn get(&self, path: &str) -> Result<(Vec<u8>, Option<String>), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .get(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV GET failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Err("NotFound".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV GET returned {status}"));
        }

        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        Ok((body.to_vec(), etag))
    }

    pub async fn put(&self, path: &str, data: &[u8]) -> Result<(), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .put(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| format!("WebDAV PUT failed: {e}"))?;

        let status = response.status();
        if !status.is_success() && status != StatusCode::CREATED {
            return Err(format!("WebDAV PUT returned {status}"));
        }
        Ok(())
    }

    pub async fn put_with_etag(&self, path: &str, data: &[u8], etag: &str) -> Result<PutResult, String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .put(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/octet-stream")
            .header("If-Match", etag)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| format!("WebDAV PUT failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::PRECONDITION_FAILED {
            return Ok(PutResult::EtagConflict);
        }
        if !status.is_success() && status != StatusCode::CREATED {
            return Err(format!("WebDAV PUT returned {status}"));
        }
        Ok(PutResult::Ok)
    }

    pub async fn mkcol(&self, path: &str) -> Result<(), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV MKCOL failed: {e}"))?;

        let status = response.status();
        // 201 Created or 405 Method Not Allowed (already exists) are both OK
        if status == StatusCode::CREATED || status == StatusCode::METHOD_NOT_ALLOWED {
            return Ok(());
        }
        if !status.is_success() {
            return Err(format!("WebDAV MKCOL returned {status}"));
        }
        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<(), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .delete(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV DELETE failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(()); // Already gone
        }
        if !status.is_success() && status != StatusCode::NO_CONTENT {
            return Err(format!("WebDAV DELETE returned {status}"));
        }
        Ok(())
    }

    pub async fn exists(&self, path: &str) -> Result<bool, String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .head(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV HEAD failed: {e}"))?;

        Ok(response.status().is_success())
    }

    pub async fn test_connection(&self) -> Result<(), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .request(
                reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
                &self.base_url,
            )
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "0")
            .send()
            .await
            .map_err(|e| format!("WebDAV connection test failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err("Authentication failed — check username and password".to_string());
        }
        // 207 Multi-Status is the expected PROPFIND response
        if status.as_u16() == 207 || status.is_success() {
            info!("WebDAV connection test successful");
            return Ok(());
        }
        Err(format!("WebDAV server returned unexpected status: {status}"))
    }

    pub async fn ensure_directory_structure(&self, remote_path: &str) -> Result<(), String> {
        let path = remote_path.trim_matches('/');
        self.mkcol(path).await?;
        self.mkcol(&format!("{}/meta", path)).await?;
        self.mkcol(&format!("{}/entries", path)).await?;
        info!("WebDAV directory structure ensured at /{}", path);
        Ok(())
    }
}
