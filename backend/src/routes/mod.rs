// Core
use axum::{routing::post, Router};

// Services
mod chat;
mod generate;

pub fn router() -> Router {
    Router::new()
        .route("/chat", post(chat::chat_handler))
        .route("/generate-page", post(generate::generate_page_handler))
}
