use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteManifestPlatform {
    pub signature: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteManifest {
    pub version: String,
    pub notes: Option<String>,
    #[serde(rename = "pub_date")]
    pub pub_date: Option<String>,
    pub platforms: HashMap<String, RemoteManifestPlatform>,
}

impl RemoteManifest {
    pub fn parse(input: &str) -> Result<Self, String> {
        serde_json::from_str(input).map_err(|e| e.to_string())
    }

    pub fn platform(&self, target: &str) -> Option<&RemoteManifestPlatform> {
        self.platforms.get(target)
    }
}
