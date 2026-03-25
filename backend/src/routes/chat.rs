// Core
use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub history: Vec<HistoryMessage>,
    pub model: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub reply: String,
}

pub async fn chat_handler(
    Json(payload): Json<ChatRequest>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let api_key = std::env::var("GOOGLE_AI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "GOOGLE_AI_API_KEY not set"})),
        )
    })?;

    let model = payload.model.unwrap_or_else(|| "gemini-2.5-flash".to_string());

    // Build contents array from history + new message
    let mut contents: Vec<serde_json::Value> = payload
        .history
        .iter()
        .map(|msg| {
            json!({
                "role": if msg.role == "assistant" { "model" } else { "user" },
                "parts": [{ "text": msg.content }]
            })
        })
        .collect();

    contents.push(json!({
        "role": "user",
        "parts": [{ "text": payload.message }]
    }));

    let request_body = json!({
        "contents": contents,
        "generationConfig": {
            "temperature": 0.7,
            "maxOutputTokens": 2048
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("Failed to call Gemini API: {}", e)})),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("Gemini API error {}: {}", status, error_text)})),
        ));
    }

    let gemini_response: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to parse response: {}", e)})),
        )
    })?;

    let reply = gemini_response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("No response")
        .to_string();

    Ok(Json(ChatResponse { reply }).into_response())
}
