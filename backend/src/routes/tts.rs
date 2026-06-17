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

#[derive(Deserialize)]
pub struct TestVoiceRequest {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub speed: Option<f32>,
}

/// XTTS v2 supported languages, surfaced to the frontend voice/engine pickers.
const XTTS_LANGUAGES: [&str; 17] = [
    "en", "es", "fr", "de", "it", "pt", "pl", "tr", "ru", "nl", "cs", "ar", "zh-cn", "ja", "hu",
    "ko", "hi",
];

const RU_VOICE_TEST_TEXT: &str =
    "Это тест русской озвучки. Если голос звучит хорошо, можно начинать генерацию книги.";

fn resolve_provider() -> &'static str {
    let raw = env::var("TTS_PROVIDER")
        .unwrap_or_else(|_| "local".to_string())
        .to_lowercase();
    if raw.starts_with("local") || raw.starts_with("xtts") || raw.starts_with("coqui") {
        "local"
    } else if raw.starts_with("google") || raw.starts_with("vertex") {
        "google"
    } else if raw.starts_with("resemble") {
        "resemble"
    } else if raw.starts_with("eleven") {
        "elevenlabs"
    } else {
        "local"
    }
}

fn local_base_url() -> String {
    env::var("TTS_LOCAL_URL").unwrap_or_else(|_| "http://127.0.0.1:8123".to_string())
}

fn local_language() -> String {
    env::var("XTTS_LANGUAGE").unwrap_or_else(|_| "ru".to_string())
}

fn local_speaker() -> String {
    env::var("XTTS_SPEAKER").unwrap_or_else(|_| "Viktor Eka".to_string())
}

fn local_speed() -> f32 {
    env::var("XTTS_SPEED")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1.0)
}

/// Shape the frontend already expects: base64 audio + (empty) alignment.
fn audio_payload(audio_base64: String) -> serde_json::Value {
    serde_json::json!({
        "audio_base64": audio_base64,
        "alignment": {
            "character_start_times_seconds": [],
            "character_end_times_seconds": []
        }
    })
}

pub async fn tts_handler(
    Json(payload): Json<TtsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if payload.text.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Text is empty".to_string()));
    }

    // Strip tables, code, special characters and dots/ellipses before synthesis,
    // so the engine reads cleaner prose and we send fewer characters per request.
    let text = crate::normalize::sanitize_for_speech(&payload.text);
    if text.trim().is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "No speakable text after filtering".to_string(),
        ));
    }

    let provider = resolve_provider();

    // Cache check
    let mut hasher = Sha256::new();
    hasher.update(provider.as_bytes());
    hasher.update(text.as_bytes());

    if provider == "local" {
        hasher.update(local_language().as_bytes());
        hasher.update(local_speaker().as_bytes());
        hasher.update(local_speed().to_le_bytes());
    } else if provider == "google" {
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
    let json_resp = if provider == "local" {
        synthesize_local(
            &client,
            &local_base_url(),
            &text,
            &local_language(),
            &local_speaker(),
            local_speed(),
        )
        .await?
    } else if provider == "google" {
        synthesize_google_tts(&client, &text).await?
    } else if provider == "resemble" {
        match synthesize_resemble_tts(&client, &text).await {
            Ok(v) => v,
            Err((status, msg)) => {
                if msg.to_lowercase().contains("quota") {
                    synthesize_google_tts(&client, &text).await?
                } else {
                    return Err((status, msg));
                }
            }
        }
    } else {
        match synthesize_elevenlabs_tts(&client, &text).await {
            Ok(v) => v,
            Err((status, msg)) => {
                if msg.to_lowercase().contains("quota_exceeded") {
                    synthesize_google_tts(&client, &text).await?
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

/// Call the local XTTS v2 sidecar (`tts_service/`) and return base64 WAV audio.
async fn synthesize_local(
    client: &Client,
    base_url: &str,
    text: &str,
    language: &str,
    speaker: &str,
    speed: f32,
) -> Result<serde_json::Value, (StatusCode, String)> {
    let url = format!("{}/synthesize", base_url.trim_end_matches('/'));
    let req_body = serde_json::json!({
        "text": text,
        "language": language,
        "speaker": speaker,
        "speed": speed,
    });

    let res = client
        .post(url)
        .json(&req_body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Local TTS sidecar unreachable: {}", e),
            )
        })?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Local TTS Error: {}", err_text),
        ));
    }

    let audio_bytes = res
        .bytes()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let audio_base64 = general_purpose::STANDARD.encode(audio_bytes);

    Ok(audio_payload(audio_base64))
}

/// GET /api/tts/engines — engines available to the frontend.
pub async fn list_engines() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "engines": [
            {
                "id": "local",
                "name": "Coqui XTTS v2 (local)",
                "languages": XTTS_LANGUAGES,
                "active": resolve_provider() == "local"
            }
        ]
    }))
}

/// GET /api/tts/voices — proxy the local sidecar's voice list.
pub async fn list_voices() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let client = Client::new();
    let url = format!("{}/voices", local_base_url().trim_end_matches('/'));
    let res = client.get(url).send().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Local TTS sidecar unreachable: {}", e),
        )
    })?;
    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((StatusCode::INTERNAL_SERVER_ERROR, err_text));
    }
    let value: serde_json::Value = res
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(value))
}

/// POST /api/tts/test-voice — synthesize a short sample (defaults to the RU test phrase).
pub async fn test_voice(
    Json(payload): Json<TestVoiceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let text = payload
        .text
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| RU_VOICE_TEST_TEXT.to_string());
    let language = payload.language.unwrap_or_else(local_language);
    let speaker = payload.speaker.unwrap_or_else(local_speaker);
    let speed = payload.speed.unwrap_or_else(local_speed);

    let client = Client::new();
    let value = synthesize_local(&client, &local_base_url(), &text, &language, &speaker, speed).await?;
    Ok(Json(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn local_synthesis_returns_base64_wav() {
        let server = MockServer::start().await;
        let wav: &[u8] = b"RIFF....WAVEfake-bytes";

        Mock::given(method("POST"))
            .and(path("/synthesize"))
            .and(body_json(serde_json::json!({
                "text": "Привет",
                "language": "ru",
                "speaker": "Ana Florence",
                "speed": 1.0
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "audio/wav")
                    .set_body_bytes(wav.to_vec()),
            )
            .mount(&server)
            .await;

        let client = Client::new();
        let value = synthesize_local(&client, &server.uri(), "Привет", "ru", "Ana Florence", 1.0)
            .await
            .expect("synthesis should succeed");

        let b64 = value["audio_base64"].as_str().expect("audio_base64 present");
        let decoded = general_purpose::STANDARD.decode(b64).expect("valid base64");
        assert_eq!(decoded, wav);
        // alignment is present (empty) so the existing frontend shape holds
        assert!(value["alignment"]["character_start_times_seconds"].is_array());
    }

    #[tokio::test]
    async fn local_synthesis_propagates_sidecar_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/synthesize"))
            .respond_with(ResponseTemplate::new(500).set_body_string("model not loaded"))
            .mount(&server)
            .await;

        let client = Client::new();
        let err = synthesize_local(&client, &server.uri(), "x", "ru", "spk", 1.0)
            .await
            .expect_err("should propagate error");
        assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(err.1.contains("model not loaded"));
    }

    #[tokio::test]
    async fn local_synthesis_reports_unreachable_sidecar() {
        let client = Client::new();
        // Nothing is listening on this port.
        let err = synthesize_local(&client, "http://127.0.0.1:1", "x", "ru", "spk", 1.0)
            .await
            .expect_err("should fail to connect");
        assert_eq!(err.0, StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn resolve_provider_defaults_to_local() {
        // Aliases map onto the local engine.
        for raw in ["local", "xtts-v2", "coqui"] {
            std::env::set_var("TTS_PROVIDER", raw);
            assert_eq!(resolve_provider(), "local");
        }
        std::env::set_var("TTS_PROVIDER", "google");
        assert_eq!(resolve_provider(), "google");
        std::env::remove_var("TTS_PROVIDER");
        assert_eq!(resolve_provider(), "local");
    }
}
