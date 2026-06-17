//! Synthesizer abstraction used by the generation pipeline.
//!
//! The pipeline is generic over [`SpeechSynthesizer`] so it can be driven by a
//! fake in tests and by the real local XTTS sidecar in production.

// Core
use std::env;
use std::future::Future;
use reqwest::Client;

/// Turns text into WAV-encoded audio bytes.
pub trait SpeechSynthesizer {
    fn synthesize(
        &self,
        text: &str,
    ) -> impl Future<Output = Result<Vec<u8>, String>> + Send;
}

/// Calls the local XTTS v2 sidecar (`tts_service/`).
pub struct LocalSynthesizer {
    client: Client,
    base_url: String,
    language: String,
    speaker: String,
    speed: f32,
}

impl LocalSynthesizer {
    pub fn from_env() -> Self {
        Self {
            client: Client::new(),
            base_url: env::var("TTS_LOCAL_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8123".to_string()),
            language: env::var("XTTS_LANGUAGE").unwrap_or_else(|_| "ru".to_string()),
            speaker: env::var("XTTS_SPEAKER").unwrap_or_else(|_| "Ana Florence".to_string()),
            speed: env::var("XTTS_SPEED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
        }
    }
}

impl SpeechSynthesizer for LocalSynthesizer {
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
        let url = format!("{}/synthesize", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "text": text,
            "language": self.language,
            "speaker": self.speaker,
            "speed": self.speed,
        });
        let res = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("sidecar unreachable: {e}"))?;
        if !res.status().is_success() {
            let detail = res.text().await.unwrap_or_default();
            return Err(format!("sidecar error: {detail}"));
        }
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        Ok(bytes.to_vec())
    }
}
