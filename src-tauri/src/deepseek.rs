//! Client for the Virex API proxy.
//!
//! The proxy (a Cloudflare Worker) holds the DeepSeek key and enforces the plan
//! limits, so nothing secret ships in this binary. It streams DeepSeek's SSE
//! response straight through, which is why the parsing below is unchanged from
//! talking to DeepSeek directly.

use std::time::Duration;

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

use crate::settings::Settings;

#[derive(Debug, Error)]
pub enum DeepSeekError {
    #[error("request timed out")]
    Timeout,
    #[error("network error: {0}")]
    Network(String),
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("malformed response from API")]
    Parse,
    #[error("{0}")]
    DailyLimit(String),
}

/// Parameters for a single rewrite request.
pub struct RewriteRequest<'a> {
    pub settings: &'a Settings,
    pub input: &'a str,
    /// Anonymous per-install id the proxy counts free usage against.
    pub device_id: &'a str,
    /// Pro licence, when the user has one.
    pub license: Option<&'a str>,
}

// ---- Minimal SSE payload shapes ------------------------------------------

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
}

/// Stream a rewrite. `on_delta` is invoked for every content fragment as it
/// arrives; the accumulated full text is returned on success.
///
/// The prompt text is intentionally never logged.
pub async fn stream_rewrite<F>(
    req: RewriteRequest<'_>,
    mut on_delta: F,
) -> Result<String, DeepSeekError>
where
    F: FnMut(&str),
{
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(req.settings.timeout_secs.max(5)))
        .build()
        .map_err(|e| DeepSeekError::Network(e.to_string()))?;

    let url = format!("{}/v1/rewrite", req.settings.api_base().trim_end_matches('/'));

    let body = json!({
        "text": req.input,
        "system": req.settings.system_prompt,
    });

    let mut request = client
        .post(&url)
        .header("X-Virex-Device", req.device_id)
        .json(&body);
    if let Some(license) = req.license.filter(|l| !l.trim().is_empty()) {
        request = request.bearer_auth(license);
    }

    let resp = request.send().await.map_err(|e| {
        if e.is_timeout() {
            DeepSeekError::Timeout
        } else {
            DeepSeekError::Network(e.to_string())
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let raw = resp.text().await.unwrap_or_default();

        // The proxy reports an exhausted free allowance as 429; surface its
        // message verbatim so the overlay can prompt an upgrade.
        if status == 429 {
            let message = serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
                .unwrap_or_else(|| "You've hit today's free limit. Upgrade to Pro for unlimited rewrites.".into());
            return Err(DeepSeekError::DailyLimit(message));
        }

        let message = raw.chars().take(300).collect();
        return Err(DeepSeekError::Api { status, message });
    }

    let mut stream = resp.bytes_stream().eventsource();
    let mut full = String::new();

    while let Some(event) = stream.next().await {
        let event = event.map_err(|e| DeepSeekError::Network(e.to_string()))?;
        if event.data == "[DONE]" {
            break;
        }
        // Skip keep-alive / non-JSON frames gracefully.
        let chunk: StreamChunk = match serde_json::from_str(&event.data) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(choice) = chunk.choices.into_iter().next() {
            if let Some(content) = choice.delta.content {
                if !content.is_empty() {
                    full.push_str(&content);
                    on_delta(&content);
                }
            }
        }
    }

    if full.is_empty() {
        return Err(DeepSeekError::Parse);
    }
    Ok(full)
}
