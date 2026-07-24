//! DeepSeek chat-completions client (OpenAI-compatible), with streaming.

use std::time::Duration;

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::settings::Settings;

#[derive(Debug, Error)]
pub enum DeepSeekError {
    #[error("no API key configured — add one in Settings")]
    MissingKey,
    #[error("request timed out")]
    Timeout,
    #[error("network error: {0}")]
    Network(String),
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("malformed response from API")]
    Parse,
}

/// Parameters for a single rewrite request.
pub struct RewriteRequest<'a> {
    pub api_key: &'a str,
    pub settings: &'a Settings,
    pub input: &'a str,
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

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
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
    if req.api_key.trim().is_empty() {
        return Err(DeepSeekError::MissingKey);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(req.settings.timeout_secs.max(5)))
        .build()
        .map_err(|e| DeepSeekError::Network(e.to_string()))?;

    let url = format!(
        "{}/chat/completions",
        req.settings.base_url.trim_end_matches('/')
    );

    let body = json!({
        "model": req.settings.model,
        "temperature": req.settings.temperature,
        "stream": true,
        "messages": [
            Message { role: "system", content: &req.settings.system_prompt },
            Message { role: "user", content: req.input },
        ],
    });

    let resp = client
        .post(&url)
        .bearer_auth(req.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                DeepSeekError::Timeout
            } else {
                DeepSeekError::Network(e.to_string())
            }
        })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let message = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(300)
            .collect();
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
