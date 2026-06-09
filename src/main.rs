mod api;
mod config;
mod engine;
mod label;
mod usb;

use std::sync::Arc;
use tokio::sync::RwLock;

use axum::{Router, response::{Html, IntoResponse}, routing::get};
use rust_embed::Embed;
use tower_http::cors::CorsLayer;

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

pub struct AppState {
    pub config: config::AppConfig,
    pub devices: Vec<config::DeviceDef>,
    pub labels: Vec<label::LabelDef>,
    pub fonts: engine::text::FontStore,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "labelmanagerpnp=debug,tower_http=debug".into()),
        )
        .init();

    let config = config::AppConfig::load_or_default("config.toml");
    let devices = config::load_devices("devices/");
    let labels = label::load_labels("labels/");
    let fonts = engine::text::FontStore::load(
        "fonts/",
        &config.font_favourites_medium,
        &config.font_favourites_small,
        config.show_all_fonts,
    );

    tracing::info!("Loaded {} device definitions", devices.len());
    tracing::info!("Loaded {} label definitions", labels.len());
    let groups = fonts.groups();
    tracing::info!(
        "Loaded fonts: {} medium, {} small, {} system",
        groups.medium.len(),
        groups.small.len(),
        groups.system.len(),
    );

    let state = Arc::new(RwLock::new(AppState { config, devices, labels, fonts }));

    let app = Router::new()
        .route("/", get(index_handler))
        .nest("/api", api::router())
        .fallback(get(static_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let bind = "0.0.0.0:3000";
    tracing::info!("Listening on http://{bind}");

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> Html<String> {
    match StaticAssets::get("index.html") {
        Some(content) => Html(String::from_utf8_lossy(&content.data).to_string()),
        None => Html("<h1>index.html not found</h1>".to_string()),
    }
}

async fn static_handler(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    let path = uri.path().trim_start_matches('/');

    match StaticAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}
