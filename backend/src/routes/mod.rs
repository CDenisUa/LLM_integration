// Core
use axum::{routing::post, Router};
// Services
mod chat;
mod generate;
mod pipeline;
mod tts;
mod pdf;

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
        .route("/tts", post(tts::tts_handler))
        .route("/pdf", axum::routing::get(pdf::list_pdfs))
        .route("/pdf/:id", axum::routing::put(pdf::update_pdf))
        .route("/pdf/upload", post(pdf::upload_pdf))
        .nest_service("/pdf/files", tower_http::services::ServeDir::new("storage/pdfs"))
        .nest_service("/pdf/covers", tower_http::services::ServeDir::new("storage/covers"))
}
