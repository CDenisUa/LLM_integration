use axum::{http::StatusCode, Json};
use serde::Deserialize;
use reqwest::Client;
use std::env;
use sha2::{Sha256, Digest};
use tokio::fs;
use base64::{engine::general_purpose, Engine as _};

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

    let provider_raw = env::var("TTS_PROVIDER")
        .unwrap_or_else(|_| "google".to_string())
        .to_lowercase();
    let provider = if provider_raw.starts_with("google") || provider_raw.starts_with("vertex") {
        "google"
    } else if provider_raw.starts_with("resemble") {
        "resemble"
    } else if provider_raw.starts_with("eleven") {
        "elevenlabs"
    } else {
        "google"
    };

    // Cache check
    let mut hasher = Sha256::new();
    hasher.update(provider.as_bytes());
    hasher.update(payload.text.as_bytes());

    if provider == "google" {
        let language_code = env::var("GOOGLE_TTS_LANGUAGE_CODE")
            .unwrap_or_else(|_| "ru-RU".to_string());
        let voice_name = env::var("GOOGLE_TTS_VOICE")
            .unwrap_or_else(|_| "ru-RU-Chirp3-HD-Aoede".to_string());
        let speaking_rate = env::var("GOOGLE_TTS_SPEAKING_RATE")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.0);

        hasher.update(language_code.as_bytes());
        hasher.update(voice_name.as_bytes());
        hasher.update(speaking_rate.to_le_bytes());
    } else if provider == "elevenlabs" {
        let voice_id = env::var("ELEVENLABS_VOICE_ID")
            .unwrap_or_else(|_| "pNInz6obpgDQGcFmaJgB".to_string());
        let model_id = env::var("ELEVENLABS_MODEL_ID")
            .unwrap_or_else(|_| "eleven_multilingual_v2".to_string());
        hasher.update(voice_id.as_bytes());
        hasher.update(model_id.as_bytes());
    } else {
        let resemble_voice_uuid = env::var("RESEMBLE_VOICE_UUID").unwrap_or_default();
        let resemble_project_uuid = env::var("RESEMBLE_PROJECT_UUID").unwrap_or_default();
        let resemble_precision = env::var("RESEMBLE_PRECISION")
            .unwrap_or_else(|_| "MP3".to_string());
        hasher.update(resemble_voice_uuid.as_bytes());
        hasher.update(resemble_project_uuid.as_bytes());
        hasher.update(resemble_precision.as_bytes());
    }

    let hash = format!("{:x}", hasher.finalize());
    let cache_path = format!("storage/tts/{}.json", hash);

    if let Ok(cached_data) = fs::read_to_string(&cache_path).await {
        if let Ok(json_resp) = serde_json::from_str::<serde_json::Value>(&cached_data) {
            return Ok(Json(json_resp));
        }
    }

    let client = Client::new();
    let json_resp = if provider == "google" {
        synthesize_google_tts(&client, &payload.text).await?
    } else if provider == "resemble" {
        match synthesize_resemble_tts(&client, &payload.text).await {
            Ok(v) => v,
            Err((status, msg)) => {
                if msg.to_lowercase().contains("quota") {
                    synthesize_google_tts(&client, &payload.text).await?
                } else {
                    return Err((status, msg));
                }
            }
        }
    } else {
        match synthesize_elevenlabs_tts(&client, &payload.text).await {
            Ok(v) => v,
            Err((status, msg)) => {
                if msg.to_lowercase().contains("quota_exceeded") {
                    synthesize_google_tts(&client, &payload.text).await?
                } else {
                    return Err((status, msg));
                }
            }
        }
    };

    // Save cache
    if let Ok(json_str) = serde_json::to_string(&json_resp) {
        let _ = fs::write(&cache_path, json_str).await;
    }

    Ok(Json(json_resp))
}

async fn synthesize_google_tts(
    client: &Client,
    text: &str,
) -> Result<serde_json::Value, (StatusCode, String)> {
    let api_key = env::var("GOOGLE_TEXT_TO_SPEECH_API_KEY")
        .or_else(|_| env::var("GOOGLE_TTS_API_KEY"))
        .or_else(|_| env::var("GOOGLE_AI_API_KEY"))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "GOOGLE_TEXT_TO_SPEECH_API_KEY (or GOOGLE_TTS_API_KEY / GOOGLE_AI_API_KEY) is not set".to_string(),
            )
        })?;

    let language_code = env::var("GOOGLE_TTS_LANGUAGE_CODE")
        .unwrap_or_else(|_| "ru-RU".to_string());
    let voice_name = env::var("GOOGLE_TTS_VOICE")
        .unwrap_or_else(|_| "ru-RU-Chirp3-HD-Aoede".to_string());
    let speaking_rate = env::var("GOOGLE_TTS_SPEAKING_RATE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1.0);
    let pitch = env::var("GOOGLE_TTS_PITCH")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0);

    let url = format!(
        "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
        api_key
    );
    let req_body = serde_json::json!({
        "input": { "text": text },
        "voice": {
            "languageCode": language_code,
            "name": voice_name
        },
        "audioConfig": {
            "audioEncoding": "MP3",
            "speakingRate": speaking_rate,
            "pitch": pitch
        }
    });

    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&req_body)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Google TTS Error: {}", err_text),
        ));
    }

    let value: serde_json::Value = res
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let audio_base64 = value
        .get("audioContent")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Google TTS response has no audioContent".to_string(),
            )
        })?;

    // Keep the same payload shape expected by the frontend.
    Ok(serde_json::json!({
        "audio_base64": audio_base64,
        "alignment": {
            "character_start_times_seconds": [],
            "character_end_times_seconds": []
        }
    }))
}

async fn synthesize_resemble_tts(
    client: &Client,
    text: &str,
) -> Result<serde_json::Value, (StatusCode, String)> {
    let api_key = env::var("RESEMBLE_AI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "RESEMBLE_AI_API_KEY is not set".to_string(),
        )
    })?;

    let url = env::var("RESEMBLE_SYNTHESIS_URL")
        .unwrap_or_else(|_| "https://f.cluster.resemble.ai/synthesize".to_string());

    let language_code = env::var("GOOGLE_TTS_LANGUAGE_CODE")
        .unwrap_or_else(|_| "ru-RU".to_string());
    
    // Wrap text in SSML to enforce the desired accent/language
    let ssml_text = format!("<speak><lang xml:lang=\"{}\">{}</lang></speak>", language_code, text);

    let mut req_body = serde_json::Map::new();
    req_body.insert("data".to_string(), serde_json::Value::String(ssml_text));

    if let Ok(v) = env::var("RESEMBLE_VOICE_UUID") {
        if !v.trim().is_empty() {
            req_body.insert("voice_uuid".to_string(), serde_json::Value::String(v));
        }
    }
    if let Ok(v) = env::var("RESEMBLE_PROJECT_UUID") {
        if !v.trim().is_empty() {
            req_body.insert("project_uuid".to_string(), serde_json::Value::String(v));
        }
    }
    if let Ok(v) = env::var("RESEMBLE_PRECISION") {
        if !v.trim().is_empty() {
            req_body.insert("precision".to_string(), serde_json::Value::String(v));
        }
    }
    if let Ok(v) = env::var("RESEMBLE_SAMPLE_RATE") {
        if let Ok(rate) = v.parse::<u32>() {
            req_body.insert(
                "sample_rate".to_string(),
                serde_json::Value::Number(serde_json::Number::from(rate)),
            );
        }
    }

    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&req_body)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Resemble Error: {}", err_text),
        ));
    }

    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if content_type.contains("application/json") {
        let value: serde_json::Value = res
            .json()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut audio_base64 = value
            .get("audio_base64")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if audio_base64.is_none() {
            audio_base64 = value
                .get("audio")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        if audio_base64.is_none() {
            audio_base64 = value
                .get("audio_content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        if audio_base64.is_none() {
            if let Some(audio_url) = value
                .get("url")
                .and_then(|v| v.as_str())
                .or_else(|| value.get("audio_url").and_then(|v| v.as_str()))
            {
                let audio_bytes = client
                    .get(audio_url)
                    .send()
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                audio_base64 = Some(general_purpose::STANDARD.encode(audio_bytes));
            }
        }

        let audio_base64 = audio_base64.ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Resemble TTS response has no audio payload".to_string(),
            )
        })?;

        return Ok(serde_json::json!({
            "audio_base64": audio_base64,
            "alignment": {
                "character_start_times_seconds": [],
                "character_end_times_seconds": []
            }
        }));
    }

    let audio_bytes = res
        .bytes()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let audio_base64 = general_purpose::STANDARD.encode(audio_bytes);

    Ok(serde_json::json!({
        "audio_base64": audio_base64,
        "alignment": {
            "character_start_times_seconds": [],
            "character_end_times_seconds": []
        }
    }))
}

async fn synthesize_elevenlabs_tts(
    client: &Client,
    text: &str,
) -> Result<serde_json::Value, (StatusCode, String)> {
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

    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}/with-timestamps",
        voice_id
    );

    let elevenlabs_req = serde_json::json!({
        "text": text,
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

    res.json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
