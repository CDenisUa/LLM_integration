// Core
use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ── Shared helpers ────────────────────────────────────────────────────────────

fn api_key() -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    std::env::var("GOOGLE_AI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "GOOGLE_AI_API_KEY not set"})),
        )
    })
}

async fn gemini(prompt: &str) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let key = api_key()?;
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        key
    );
    let body = json!({
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {"temperature": 0.7, "maxOutputTokens": 4096}
    });
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err((StatusCode::BAD_GATEWAY, Json(json!({"error": text}))));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    Ok(json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

// ── Generate image prompt ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ImagePromptRequest {
    pub product_name: String,
    pub product_type: String,
    pub audience: String,
    pub brand_style: String,
}

#[derive(Serialize)]
pub struct ImagePromptResponse {
    pub prompt: String,
}

pub async fn generate_image_prompt_handler(
    Json(payload): Json<ImagePromptRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let prompt = format!(
        "Generate a short, vivid image generation prompt (max 2 sentences) for a product photo. \
        Product: \"{}\", type: \"{}\", target audience: \"{}\", brand style: \"{}\". \
        The image should show a model or person using/wearing the product in a compelling lifestyle scenario. \
        Return ONLY the prompt text, no explanations.",
        payload.product_name, payload.product_type, payload.audience, payload.brand_style
    );

    let result = gemini(&prompt).await?;
    Ok(Json(ImagePromptResponse { prompt: result.trim().to_string() }))
}

// ── Generate image (Imagen) ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GenerateImageRequest {
    pub prompt: String,
}

#[derive(Serialize)]
pub struct GenerateImageResponse {
    pub images: Vec<String>,
}

pub async fn generate_image_handler(
    Json(payload): Json<GenerateImageRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let key = api_key()?;
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict?key={}",
        key
    );
    let body = json!({
        "instances": [{"prompt": payload.prompt}],
        "parameters": {"sampleCount": 2, "aspectRatio": "3:4"}
    });

    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err((StatusCode::BAD_GATEWAY, Json(json!({"error": text}))));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    let images: Vec<String> = json["predictions"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|p| p["bytesBase64Encoded"].as_str())
        .map(|s| format!("data:image/png;base64,{}", s))
        .collect();

    Ok(Json(GenerateImageResponse { images }))
}

// ── Generate SEO ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GenerateSeoRequest {
    pub product_name: String,
    pub product_type: String,
    pub audience: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct GenerateSeoResponse {
    pub h1: String,
    pub title: String,
    pub meta_description: String,
    pub alt_text: String,
}

pub async fn generate_seo_handler(
    Json(payload): Json<GenerateSeoRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let prompt = format!(
        "Generate SEO content for a landing page. Return ONLY a JSON object with these exact keys: \
        \"h1\", \"title\", \"meta_description\", \"alt_text\". \
        Product: \"{}\", type: \"{}\", audience: \"{}\", description: \"{}\". \
        Make it compelling, keyword-rich, and under the character limits (h1: 60, title: 60, meta: 155, alt: 125).",
        payload.product_name, payload.product_type, payload.audience, payload.description
    );

    let raw = gemini(&prompt).await?;
    let cleaned = raw.trim().trim_start_matches("```json").trim_end_matches("```").trim();

    let parsed: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to parse SEO JSON"}))))?;

    Ok(Json(GenerateSeoResponse {
        h1: parsed["h1"].as_str().unwrap_or("").to_string(),
        title: parsed["title"].as_str().unwrap_or("").to_string(),
        meta_description: parsed["meta_description"].as_str().unwrap_or("").to_string(),
        alt_text: parsed["alt_text"].as_str().unwrap_or("").to_string(),
    }))
}

// ── Generate content ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GenerateContentRequest {
    pub product_name: String,
    pub product_type: String,
    pub audience: String,
    pub brand_style: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct GenerateContentResponse {
    pub hero_headline: String,
    pub hero_subtext: String,
    pub features: Vec<String>,
    pub cta_text: String,
    pub testimonial: String,
}

pub async fn generate_content_handler(
    Json(payload): Json<GenerateContentRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let prompt = format!(
        "Generate landing page copy. Return ONLY a JSON object with these exact keys: \
        \"hero_headline\" (punchy, max 8 words), \
        \"hero_subtext\" (1-2 sentences), \
        \"features\" (array of 3 short benefit strings), \
        \"cta_text\" (action button text, max 4 words), \
        \"testimonial\" (one realistic customer quote with name). \
        Product: \"{}\", type: \"{}\", audience: \"{}\", brand style: \"{}\", description: \"{}\".",
        payload.product_name, payload.product_type, payload.audience,
        payload.brand_style, payload.description
    );

    let raw = gemini(&prompt).await?;
    let cleaned = raw.trim().trim_start_matches("```json").trim_end_matches("```").trim();

    let parsed: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to parse content JSON"}))))?;

    let features = parsed["features"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|f| f.as_str())
        .map(|s| s.to_string())
        .collect();

    Ok(Json(GenerateContentResponse {
        hero_headline: parsed["hero_headline"].as_str().unwrap_or("").to_string(),
        hero_subtext: parsed["hero_subtext"].as_str().unwrap_or("").to_string(),
        features,
        cta_text: parsed["cta_text"].as_str().unwrap_or("").to_string(),
        testimonial: parsed["testimonial"].as_str().unwrap_or("").to_string(),
    }))
}

// ── Generate styles ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GenerateStylesRequest {
    pub product_name: String,
    pub brand_style: String,
    pub product_type: String,
}

#[derive(Serialize)]
pub struct GenerateStylesResponse {
    pub primary_color: String,
    pub secondary_color: String,
    pub font_style: String,
    pub animation_style: String,
    pub reasoning: String,
}

pub async fn generate_styles_handler(
    Json(payload): Json<GenerateStylesRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let prompt = format!(
        "Suggest a design system for a landing page. Return ONLY a JSON object with: \
        \"primary_color\" (hex), \"secondary_color\" (hex), \
        \"font_style\" (one of: modern, elegant, bold, playful, minimal), \
        \"animation_style\" (one of: smooth, snappy, dramatic, subtle), \
        \"reasoning\" (1 sentence explaining the choices). \
        Product: \"{}\", brand style: \"{}\", type: \"{}\".",
        payload.product_name, payload.brand_style, payload.product_type
    );

    let raw = gemini(&prompt).await?;
    let cleaned = raw.trim().trim_start_matches("```json").trim_end_matches("```").trim();

    let parsed: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to parse styles JSON"}))))?;

    Ok(Json(GenerateStylesResponse {
        primary_color: parsed["primary_color"].as_str().unwrap_or("#6366f1").to_string(),
        secondary_color: parsed["secondary_color"].as_str().unwrap_or("#f59e0b").to_string(),
        font_style: parsed["font_style"].as_str().unwrap_or("modern").to_string(),
        animation_style: parsed["animation_style"].as_str().unwrap_or("smooth").to_string(),
        reasoning: parsed["reasoning"].as_str().unwrap_or("").to_string(),
    }))
}

// ── Assemble final HTML ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AssembleRequest {
    pub product_name: String,
    pub selected_image: String,
    pub seo: serde_json::Value,
    pub content: serde_json::Value,
    pub styles: serde_json::Value,
}

#[derive(Serialize)]
pub struct AssembleResponse {
    pub html: String,
}

pub async fn assemble_handler(
    Json(payload): Json<AssembleRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let has_image = !payload.selected_image.is_empty();
    let image_tag = if has_image {
        format!(
            "<img src=\"{}\" alt=\"{}\" style=\"width:100%;height:100%;object-fit:cover;\"/>",
            payload.selected_image,
            payload.seo["alt_text"].as_str().unwrap_or("")
        )
    } else {
        String::new()
    };

    let features_html = payload.content["features"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|f| format!(
            "<div style=\"padding:20px;background:rgba(255,255,255,0.05);border-radius:12px;border:1px solid rgba(255,255,255,0.1)\">\
            <p style=\"margin:0;font-size:1rem;\">{}</p></div>",
            f.as_str().unwrap_or("")
        ))
        .collect::<Vec<_>>()
        .join("");

    let primary = payload.styles["primary_color"].as_str().unwrap_or("#6366f1");
    let secondary = payload.styles["secondary_color"].as_str().unwrap_or("#f59e0b");
    let animation = payload.styles["animation_style"].as_str().unwrap_or("smooth");
    let transition_speed = match animation {
        "snappy" => "0.15s",
        "dramatic" => "0.6s",
        "subtle" => "0.4s",
        _ => "0.3s",
    };

    let prompt = format!(
        "Generate a complete, beautiful, production-ready HTML landing page. \
        Return ONLY raw HTML, no markdown, no code blocks. \
        \nProduct: \"{product}\"\
        \nH1: \"{h1}\"\
        \nPage title: \"{title}\"\
        \nMeta description: \"{meta}\"\
        \nHero headline: \"{hero_h}\"\
        \nHero subtext: \"{hero_s}\"\
        \nFeatures HTML snippet (embed as-is): {features}\
        \nCTA button text: \"{cta}\"\
        \nTestimonial: \"{testimonial}\"\
        \nImage tag HTML (embed in hero section): {image_tag}\
        \nPrimary color: {primary}\
        \nSecondary color: {secondary}\
        \nTransition speed: {speed}\
        \nUse inline CSS only. Include smooth scroll, hover effects, animations. Make it world-class.",
        product = payload.product_name,
        h1 = payload.seo["h1"].as_str().unwrap_or(""),
        title = payload.seo["title"].as_str().unwrap_or(""),
        meta = payload.seo["meta_description"].as_str().unwrap_or(""),
        hero_h = payload.content["hero_headline"].as_str().unwrap_or(""),
        hero_s = payload.content["hero_subtext"].as_str().unwrap_or(""),
        features = features_html,
        cta = payload.content["cta_text"].as_str().unwrap_or(""),
        testimonial = payload.content["testimonial"].as_str().unwrap_or(""),
        image_tag = image_tag,
        primary = primary,
        secondary = secondary,
        speed = transition_speed,
    );

    let html = gemini(&prompt).await?;
    let cleaned = html.trim().trim_start_matches("```html").trim_end_matches("```").trim().to_string();

    Ok(Json(AssembleResponse { html: cleaned }))
}
