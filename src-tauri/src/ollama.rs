//! Localhost-only Ollama API client (http://127.0.0.1:11434).

use crate::error::{SmaError, SmaResult};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<OllamaModelTag>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelTag {
    name: String,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
        }
    }

    pub async fn health(&self) -> SmaResult<bool> {
        let url = format!("{}/api/tags", self.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => Err(SmaError::OllamaUnreachable(self.base_url.clone(), e.to_string())),
        }
    }

    pub async fn list_models(&self) -> SmaResult<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| SmaError::OllamaUnreachable(self.base_url.clone(), e.to_string()))?;

        if !resp.status().is_success() {
            return Err(SmaError::OllamaApi(format!("HTTP {}", resp.status())));
        }

        let body: TagsResponse = resp
            .json()
            .await
            .map_err(|e| SmaError::OllamaApi(e.to_string()))?;

        Ok(body.models.into_iter().map(|m| m.name).collect())
    }

    /// Prefer phi3 / mistral if present; otherwise first available model.
    pub async fn resolve_model(&self, preferred: Option<&str>) -> SmaResult<String> {
        let models = self.list_models().await?;
        if models.is_empty() {
            return Err(SmaError::OllamaApi(
                "No models installed. Run: ollama pull phi3".into(),
            ));
        }

        if let Some(p) = preferred {
            if let Some(found) = models.iter().find(|m| m.starts_with(p)) {
                return Ok(found.clone());
            }
        }

        for candidate in ["phi3", "mistral", "llama3", "qwen2.5"] {
            if let Some(found) = models.iter().find(|m| m.to_lowercase().starts_with(candidate)) {
                return Ok(found.clone());
            }
        }

        Ok(models[0].clone())
    }

    pub async fn generate(&self, model: &str, prompt: &str) -> SmaResult<String> {
        let url = format!("{}/api/generate", self.base_url);
        let body = GenerateRequest {
            model,
            prompt,
            stream: false,
            options: GenerateOptions {
                temperature: 0.2,
                num_predict: 512,
            },
        };

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| SmaError::OllamaUnreachable(self.base_url.clone(), e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(SmaError::OllamaApi(format!("{status}: {text}")));
        }

        let parsed: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| SmaError::OllamaApi(e.to_string()))?;

        Ok(parsed.response.trim().to_string())
    }
}
