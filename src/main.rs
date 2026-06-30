mod api;
mod config;
mod engine;
mod label;
mod usb;

use std::sync::Arc;
use tokio::sync::RwLock;

use axum::{Router, response::{Html, IntoResponse}, routing::get};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

static STATIC_INDEX: &str = include_str!("../static/index.html");
static STATIC_APP_JS: &str = include_str!("../static/app.js");
static STATIC_CANVAS_JS: &str = include_str!("../static/canvas-editor.js");
static STATIC_PICO_CSS: &str = include_str!("../static/pico.min.css");

pub struct AppState {
    pub config: config::AppConfig,
    pub devices: Vec<config::DeviceDef>,
    pub labels: Vec<label::LabelDef>,
    pub fonts: engine::text::FontStore,
}

#[tokio::main]
async fn main() {
    let level = if cfg!(debug_assertions) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    let config = config::AppConfig::load_or_default("config.toml");
    let devices = config::load_devices("devices/");
    let labels = label::load_labels("labels/");
    let fonts = engine::text::FontStore::load(
        "fonts/",
        &config.font_favourites,
        config.load_system_fonts,
    );

    tracing::info!("Loaded {} device definitions", devices.len());
    tracing::info!("Loaded {} label definitions", labels.len());
    let groups = fonts.groups();
    tracing::info!(
        "Loaded fonts: {} favourites, {} system",
        groups.favourites.len(),
        groups.system.len(),
    );

    let state = Arc::new(RwLock::new(AppState { config, devices, labels, fonts }));

    let app = Router::new()
        .route("/", get(index_handler))
        .nest("/api", api::router())
        .fallback(get(static_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind = "0.0.0.0:3000";
    tracing::info!("Listening on http://{bind}");

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, finishing in-flight requests...");
}

async fn index_handler() -> Html<&'static str> {
    Html(STATIC_INDEX)
}

async fn static_handler(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    let path = uri.path().trim_start_matches('/');

    let (content, mime): (&[u8], &str) = match path {
        "app.js" => (STATIC_APP_JS.as_bytes(), "application/javascript"),
        "canvas-editor.js" => (STATIC_CANVAS_JS.as_bytes(), "application/javascript"),
        "pico.min.css" => (STATIC_PICO_CSS.as_bytes(), "text/css"),
        "index.html" => (STATIC_INDEX.as_bytes(), "text/html"),
        _ => return axum::http::StatusCode::NOT_FOUND.into_response(),
    };

    (
        [(axum::http::header::CONTENT_TYPE, mime)],
        content,
    )
        .into_response()
}
