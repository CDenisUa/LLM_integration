// Core
use std::net::SocketAddr;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// Services
mod audio;
mod chapters;
mod chunking;
mod clean;
mod db;
mod extract;
mod models;
mod normalize;
mod pipeline;
mod routes;
mod state;
mod tts_client;
// Types
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    std::fs::create_dir_all("storage/pdfs").unwrap();
    std::fs::create_dir_all("storage/tts").unwrap();
    std::fs::create_dir_all("storage/covers").unwrap();

    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://storage/library.db".into());
    let db = db::connect(&database_url)
        .await
        .expect("failed to open library database");
    let state = AppState::new(db);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", routes::router())
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 100))
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Backend running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
