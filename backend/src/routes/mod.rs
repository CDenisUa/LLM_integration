// Core
use axum::{routing::post, Router};
// Services
mod chat;
mod generate;
mod pipeline;

pub fn router() -> Router {
    Router::new()
        .route("/chat", post(chat::chat_handler))
        .route("/generate-page", post(generate::generate_page_handler))
        .route("/pipeline/image-prompt", post(pipeline::generate_image_prompt_handler))
        .route("/pipeline/generate-image", post(pipeline::generate_image_handler))
        .route("/pipeline/seo", post(pipeline::generate_seo_handler))
        .route("/pipeline/content", post(pipeline::generate_content_handler))
        .route("/pipeline/styles", post(pipeline::generate_styles_handler))
        .route("/pipeline/assemble", post(pipeline::assemble_handler))
}
