// Core
use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct GeneratePageRequest {
    pub prompt: String,
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct GeneratePageResponse {
    pub html: String,
}

pub async fn generate_page_handler(
    Json(payload): Json<GeneratePageRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let api_key = std::env::var("GOOGLE_AI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "GOOGLE_AI_API_KEY not set"})),
        )
    })?;

    let model = payload.model.unwrap_or_else(|| "gemini-2.5-flash".to_string());

    let system_prompt = format!(
        "You are an expert web developer. Generate a complete, beautiful, self-contained HTML landing page based on the user's request. \
        Return ONLY the raw HTML code, no markdown, no code blocks, no explanations. \
        Use inline CSS with modern design: gradients, shadows, rounded corners, responsive layout. \
        The page must work without any external dependencies.\n\nUser request: {}",
        payload.prompt
    );

    let request_body = json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": system_prompt }]
        }],
        "generationConfig": {
            "temperature": 0.8,
            "maxOutputTokens": 8192
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

    let html = gemini_response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("<html><body><h1>Generation failed</h1></body></html>")
        .to_string();

    Ok(Json(GeneratePageResponse { html }))
}
