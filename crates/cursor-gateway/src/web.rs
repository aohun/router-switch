use serde_json::json;

use crate::{GatewayError, Result};

#[derive(Clone, Default)]
pub struct WebFetch {
    client: reqwest::Client,
}

impl WebFetch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn fetch(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .header("user-agent", "router-switch-cursor-gateway/0.1")
            .send()
            .await
            .map_err(|e| GatewayError::Provider(format!("WebFetch failed: {e}")))?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GatewayError::Provider(format!(
                "WebFetch {status}: {}",
                text.chars().take(500).collect::<String>()
            )));
        }
        Ok(text.chars().take(40_000).collect())
    }
}

#[derive(Clone, Default)]
pub struct WebSearch {
    client: reqwest::Client,
}

impl WebSearch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Lightweight HTML search fallback — not semble embeddings.
    pub async fn search(&self, query: &str) -> Result<String> {
        let encoded = urlencoding_lite(query);
        let url = format!("https://html.duckduckgo.com/html/?q={encoded}");
        let response = self
            .client
            .get(&url)
            .header("user-agent", "router-switch-cursor-gateway/0.1")
            .send()
            .await
            .map_err(|e| GatewayError::Provider(format!("WebSearch failed: {e}")))?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GatewayError::Provider(format!("WebSearch {status}")));
        }
        let snippet = text.chars().take(8_000).collect::<String>();
        Ok(json!({
            "query": query,
            "engine": "duckduckgo-html",
            "snippet": snippet,
        })
        .to_string())
    }
}

fn urlencoding_lite(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
