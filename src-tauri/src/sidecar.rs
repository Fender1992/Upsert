use serde::Serialize;
use std::sync::Arc;
use tauri::Emitter;
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::ollama::OllamaClient;

/// Progress payload emitted during model pulls.
#[derive(Debug, Clone, Serialize)]
pub struct ModelPullProgress {
    pub model: String,
    pub status: String,
    pub completed: u64,
    pub total: u64,
    pub percent: f64,
}

/// Manages the Ollama sidecar process lifecycle.
pub struct SidecarManager {
    child: Option<tauri_plugin_shell::process::CommandChild>,
    ollama: Arc<Mutex<OllamaClient>>,
}

impl SidecarManager {
    pub fn new(ollama: Arc<Mutex<OllamaClient>>) -> Self {
        Self {
            child: None,
            ollama,
        }
    }

    /// Spawn the Ollama sidecar and wait until it's ready (up to 30s).
    pub async fn start(&mut self, app_handle: &tauri::AppHandle) -> anyhow::Result<()> {
        // Don't double-start
        if self.child.is_some() && self.is_ready().await {
            return Ok(());
        }

        let shell = app_handle.shell();
        let (mut _rx, child) = shell
            .sidecar("ollama")
            .map_err(|e| anyhow::anyhow!("Failed to create sidecar command: {}", e))?
            .args(["serve"])
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn Ollama sidecar: {}", e))?;

        self.child = Some(child);

        // Poll for readiness
        for _ in 0..60 {
            if self.is_ready().await {
                log::info!("Ollama sidecar is ready");
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }

        anyhow::bail!("Ollama sidecar did not become ready within 30 seconds");
    }

    /// Stop the sidecar process.
    pub fn stop(&mut self) {
        if let Some(child) = self.child.take() {
            let _ = child.kill();
            log::info!("Ollama sidecar stopped");
        }
    }

    /// Check if Ollama is responding.
    pub async fn is_ready(&self) -> bool {
        let client = self.ollama.lock().await;
        client.check_status().await.running
    }

    /// Pull a model, streaming progress events to the frontend.
    pub async fn pull_model(
        &self,
        model_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> anyhow::Result<()> {
        let client = self.ollama.lock().await;
        client.pull_model_stream(model_name, app_handle).await
    }

    /// Ensure required models are available, pulling any that are missing.
    pub async fn ensure_models(&self, app_handle: &tauri::AppHandle) -> anyhow::Result<()> {
        let required = ["tinyllama", "llama3.2:3b", "nomic-embed-text"];
        let client = self.ollama.lock().await;

        let existing = client.list_models().await.unwrap_or_default();
        let existing_names: Vec<&str> = existing.iter().map(|m| m.name.as_str()).collect();

        for model in &required {
            let has_it = existing_names
                .iter()
                .any(|n| n == model || n.starts_with(&format!("{}:", model)));

            if !has_it {
                log::info!("Pulling missing model: {}", model);
                if let Err(e) = client.pull_model_stream(model, app_handle).await {
                    log::error!("Failed to pull model {}: {}", model, e);
                    // Emit error but continue with other models
                    let _ = app_handle.emit(
                        "model-pull-progress",
                        ModelPullProgress {
                            model: model.to_string(),
                            status: format!("error: {}", e),
                            completed: 0,
                            total: 0,
                            percent: 0.0,
                        },
                    );
                }
            } else {
                log::info!("Model {} already available", model);
            }
        }

        // Signal that all required models are checked
        let _ = app_handle.emit("models-ready", true);
        Ok(())
    }
}

impl Drop for SidecarManager {
    fn drop(&mut self) {
        self.stop();
    }
}
