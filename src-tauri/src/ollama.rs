use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::sidecar::ModelPullProgress;

// ── Ollama API types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub modified_at: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaTagsResponse {
    pub models: Vec<OllamaModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct OllamaChatStreamChunk {
    pub message: Option<ChatMessage>,
    pub done: bool,
}

#[derive(Debug, Serialize)]
pub struct OllamaEmbedRequest {
    pub model: String,
    pub input: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaEmbedResponse {
    pub embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub models: Vec<OllamaModel>,
}

// ── Client ───────────────────────────────────────────────────────────

pub struct OllamaClient {
    http: reqwest::Client,
    base_url: String,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
            base_url: "http://localhost:11434".to_string(),
        }
    }

    pub async fn check_status(&self) -> OllamaStatus {
        match self.list_models().await {
            Ok(models) => OllamaStatus {
                running: true,
                models,
            },
            Err(_) => OllamaStatus {
                running: false,
                models: vec![],
            },
        }
    }

    pub async fn list_models(&self) -> anyhow::Result<Vec<OllamaModel>> {
        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;
        let tags: OllamaTagsResponse = resp.json().await?;
        Ok(tags.models)
    }

    /// Stream a chat completion, returning the full response text.
    /// Sends incremental chunks via Tauri events.
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        app_handle: &tauri::AppHandle,
        request_id: &str,
    ) -> anyhow::Result<String> {
        let body = OllamaChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
        };

        // Use a longer timeout for the actual chat request
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let resp = client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {}: {}", status, text);
        }

        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();

        use futures_util::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result?;
            let chunk_str = String::from_utf8_lossy(&chunk_bytes);

            // Each line is a JSON object
            for line in chunk_str.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<OllamaChatStreamChunk>(line) {
                    if let Some(msg) = &parsed.message {
                        full_response.push_str(&msg.content);
                        // Emit incremental token to frontend
                        let _ = app_handle.emit(
                            &format!("chat-stream-{}", request_id),
                            &msg.content,
                        );
                    }
                    if parsed.done {
                        let _ = app_handle.emit(
                            &format!("chat-done-{}", request_id),
                            &full_response,
                        );
                    }
                }
            }
        }

        Ok(full_response)
    }

    /// Generate an embedding for the given text using the specified model.
    pub async fn embed(&self, model: &str, text: &str) -> anyhow::Result<Vec<f32>> {
        let body = OllamaEmbedRequest {
            model: model.to_string(),
            input: text.to_string(),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        let resp = client
            .post(format!("{}/api/embed", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama embed returned {}: {}", status, text);
        }

        let parsed: OllamaEmbedResponse = resp.json().await?;
        parsed
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embeddings returned"))
    }

    /// Check if a model is already available locally.
    pub async fn has_model(&self, name: &str) -> bool {
        match self.list_models().await {
            Ok(models) => models.iter().any(|m| {
                m.name == name || m.name.starts_with(&format!("{}:", name))
            }),
            Err(_) => false,
        }
    }

    /// Pull a model from Ollama registry, streaming progress via Tauri events.
    pub async fn pull_model_stream(
        &self,
        model_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> anyhow::Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .build()?;

        let resp = client
            .post(format!("{}/api/pull", self.base_url))
            .json(&serde_json::json!({ "name": model_name, "stream": true }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama pull failed ({}): {}", status, text);
        }

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result?;
            let chunk_str = String::from_utf8_lossy(&chunk_bytes);

            for line in chunk_str.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
                    let status_str = parsed["status"].as_str().unwrap_or("").to_string();
                    let completed = parsed["completed"].as_u64().unwrap_or(0);
                    let total = parsed["total"].as_u64().unwrap_or(0);
                    let percent = if total > 0 {
                        (completed as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };

                    let _ = app_handle.emit(
                        "model-pull-progress",
                        ModelPullProgress {
                            model: model_name.to_string(),
                            status: status_str,
                            completed,
                            total,
                            percent,
                        },
                    );
                }
            }
        }

        // Emit completion
        let _ = app_handle.emit(
            "model-pull-progress",
            ModelPullProgress {
                model: model_name.to_string(),
                status: "success".to_string(),
                completed: 0,
                total: 0,
                percent: 100.0,
            },
        );

        Ok(())
    }
}
