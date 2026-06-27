use reqwest;
use serde::Deserialize;
use chrono::{DateTime, Utc};
use crate::error::{RegistryError, Result};

#[derive(Debug, Deserialize)]
struct TailscaleDevice {
    id: String,
    name: String,
    addresses: Vec<String>,
    lastSeen: Option<String>,
    isOnline: bool,
}

/// Tailscale API client for fallback polling
pub struct TailscaleDiscovery {
    client: reqwest::Client,
    tailnet: String,
    api_key: String,
    base_url: String,
}

impl TailscaleDiscovery {
    pub fn new(tailnet: String) -> Self {
        let api_key = std::env::var("TAILSCALE_API_KEY")
            .unwrap_or_else(|_| String::new());
        
        Self {
            client: reqwest::Client::new(),
            tailnet: tailnet.clone(),
            api_key,
            base_url: format!("https://api.tailscale.com/api/v2/tailnet/{}/devices", tailnet),
        }
    }

    /// Poll Tailscale API for all devices
    pub async fn poll_devices(&self) -> Result<Vec<TailscaleDevice>> {
        if self.api_key.is_empty() {
            tracing::warn!("TAILSCALE_API_KEY not set, skipping Tailscale poll");
            return Ok(Vec::new());
        }

        let response = self.client
            .get(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| RegistryError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        if !response.status().is_success() {
            tracing::warn!("Tailscale API returned {}", response.status());
            return Ok(Vec::new());
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| RegistryError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let devices = data
            .get("devices")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|dev| {
                        Some(TailscaleDevice {
                            id: dev.get("id")?.as_str()?.to_string(),
                            name: dev.get("name")?.as_str()?.to_string(),
                            addresses: dev
                                .get("addresses")
                                .and_then(|a| a.as_array())?
                                .iter()
                                .filter_map(|a| a.as_str().map(String::from))
                                .collect(),
                            lastSeen: dev.get("lastSeen").and_then(|l| l.as_str().map(String::from)),
                            isOnline: dev.get("isOnline").and_then(|o| o.as_bool()).unwrap_or(false),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(devices)
    }

    /// Get online device IDs
    pub async fn get_online_device_ids(&self) -> Result<Vec<String>> {
        let devices = self.poll_devices().await?;
        Ok(devices
            .into_iter()
            .filter(|d| d.isOnline)
            .map(|d| d.name)  // Use name as node_id
            .collect())
    }
}
