use axum::{http::StatusCode, Json};
use serde::Deserialize;
use reqwest::Client;
use std::env;
use sha2::{Sha256, Digest};
use tokio::fs;

#[derive(Deserialize)]
pub struct TtsRequest {
    pub text: String,
}

pub async fn tts_handler(
    Json(payload): Json<TtsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if payload.text.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Text is empty".to_string()));
    }

    let api_key = env::var("ELEVENLABS_AI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ELEVENLABS_AI_API_KEY is not set".to_string(),
        )
    })?;
    let voice_id = env::var("ELEVENLABS_VOICE_ID")
        .unwrap_or_else(|_| "pNInz6obpgDQGcFmaJgB".to_string());
    let model_id = env::var("ELEVENLABS_MODEL_ID")
        .unwrap_or_else(|_| "eleven_multilingual_v2".to_string());

    // Cache check
    let mut hasher = Sha256::new();
    hasher.update(voice_id.as_bytes());
    hasher.update(model_id.as_bytes());
    hasher.update(payload.text.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let cache_path = format!("storage/tts/{}.json", hash);

    if let Ok(cached_data) = fs::read_to_string(&cache_path).await {
        if let Ok(json_resp) = serde_json::from_str::<serde_json::Value>(&cached_data) {
            return Ok(Json(json_resp));
        }
    }

    let client = Client::new();
    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}/with-timestamps",
        voice_id
    );

    let elevenlabs_req = serde_json::json!({
        "text": payload.text,
        "model_id": model_id,
        "voice_settings": {
            "stability": 0.5,
            "similarity_boost": 0.75
        }
    });

    let res = client
        .post(url)
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&elevenlabs_req)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("ElevenLabs Error: {}", err_text),
        ));
    }

    let json_resp: serde_json::Value = res
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Save cache
    if let Ok(json_str) = serde_json::to_string(&json_resp) {
        let _ = fs::write(&cache_path, json_str).await;
    }

    Ok(Json(json_resp))
}
