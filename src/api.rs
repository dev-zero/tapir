use std::sync::Arc;
use tokio::sync::RwLock;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;
use crate::engine::bitmap::Bitmap1Bit;
use crate::usb::{device, protocol::{PrintJob, FEED_LINES_FOR_CUT}};

type SharedState = Arc<RwLock<AppState>>;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/status", get(status))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/labels", get(list_labels))
        .route("/labels/reload", post(reload_labels))
        .route("/fonts", get(list_fonts))
        .route("/render-text", post(render_text))
        .route("/printers", get(list_printers))
        .route("/printers/{product_id}/print", post(print_bitmap))
        .route("/printers/{product_id}/feed", post(feed))
        .route("/preview", post(render_preview))
}

async fn status(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    let devices = device::enumerate_devices();
    let connected = devices.iter().find(|d| !d.needs_modeswitch);

    match connected {
        Some(dev) => {
            let tape_mm = state
                .devices
                .iter()
                .find(|d| d.product_id == dev.product_id)
                .map(|d| d.max_tape_mm);

            Json(json!({
                "connected": true,
                "device": dev.name,
                "product_id": dev.product_id,
                "tape_mm": tape_mm,
            }))
        }
        None => {
            let needs_modeswitch = devices.iter().any(|d| d.needs_modeswitch);
            Json(json!({
                "connected": false,
                "device": null,
                "tape_mm": null,
                "needs_modeswitch": needs_modeswitch,
            }))
        }
    }
}

async fn get_settings(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    Json(json!({
        "default_label": state.config.default_label,
        "default_canvas_width": state.config.default_canvas_width,
    }))
}

#[derive(Deserialize)]
struct SettingsUpdate {
    #[serde(default)]
    default_label: Option<String>,
}

async fn put_settings(
    State(state): State<SharedState>,
    Json(body): Json<SettingsUpdate>,
) -> Json<Value> {
    let mut state = state.write().await;
    if body.default_label.is_some() {
        state.config.default_label = body.default_label;
    }
    match state.config.save("config.toml") {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

async fn list_labels(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    Json(json!(state.labels))
}

async fn reload_labels(State(state): State<SharedState>) -> Json<Value> {
    let mut state = state.write().await;
    state.labels = crate::label::load_labels("labels/");
    Json(json!({"ok": true, "count": state.labels.len()}))
}

async fn list_printers(State(_state): State<SharedState>) -> Json<Value> {
    let devices = device::enumerate_devices();
    Json(json!(devices))
}

#[derive(Deserialize)]
struct PrintParams {
    #[serde(default = "default_true")]
    auto_feed: bool,
}

fn default_true() -> bool {
    true
}

async fn print_bitmap(
    State(state): State<SharedState>,
    Path(product_id): Path<u16>,
    Query(params): Query<PrintParams>,
    body: axum::body::Bytes,
) -> Json<Value> {
    let bitmap = match Bitmap1Bit::from_png(&body) {
        Ok(b) => b,
        Err(e) => return Json(json!({"ok": false, "error": format!("PNG decode failed: {e}")})),
    };

    if let Err(e) = device::check_device_access(product_id) {
        return Json(json!({"ok": false, "error": format!("{e}")}));
    }

    let state = state.read().await;
    let dev_def = state
        .devices
        .iter()
        .find(|d| d.product_id == product_id)
        .cloned();

    let (bytes_per_line, tape_color_id, synwait, min_label_feed) = match dev_def {
        Some(def) => {
            let bpl = (8u8 * def.max_tape_mm) / 12;
            (bpl, 0u8, def.synwait, def.min_label_feed_lines)
        }
        None => (8u8, 0u8, 64u16, FEED_LINES_FOR_CUT * 3),
    };

    let job = PrintJob {
        bytes_per_line,
        tape_color_id,
        synwait,
    };

    let mut encoded = job.encode(&bitmap);
    if params.auto_feed {
        let image_lines = bitmap.width as u16;
        let feed_lines = if image_lines < min_label_feed {
            min_label_feed - image_lines
        } else {
            FEED_LINES_FOR_CUT
        };
        encoded.extend_from_slice(&job.encode_feed(feed_lines));
    }

    match device::send_print_data(product_id, &encoded) {
        Ok(()) => Json(json!({"ok": true, "bytes_sent": encoded.len(), "auto_feed": params.auto_feed})),
        Err(e) => Json(json!({"ok": false, "error": format!("Print failed: {e}")})),
    }
}

async fn feed(
    State(state): State<SharedState>,
    Path(product_id): Path<u16>,
) -> Json<Value> {
    if let Err(e) = device::check_device_access(product_id) {
        return Json(json!({"ok": false, "error": format!("{e}")}));
    }

    let state = state.read().await;
    let dev_def = state
        .devices
        .iter()
        .find(|d| d.product_id == product_id)
        .cloned();

    let (bytes_per_line, synwait) = match dev_def {
        Some(def) => {
            let bpl = (8u8 * def.max_tape_mm) / 12;
            (bpl, def.synwait)
        }
        None => (8u8, 64u16),
    };

    let job = PrintJob {
        bytes_per_line,
        tape_color_id: 0,
        synwait,
    };

    let lines = FEED_LINES_FOR_CUT;
    let encoded = job.encode_feed(lines);

    match device::send_print_data(product_id, &encoded) {
        Ok(()) => Json(json!({"ok": true, "lines": lines})),
        Err(e) => Json(json!({"ok": false, "error": format!("Feed failed: {e}")})),
    }
}

async fn list_fonts(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    Json(json!(state.fonts.groups()))
}

#[derive(Deserialize)]
struct RenderTextRequest {
    text: String,
    font: String,
    font_size: u32,
    #[serde(default = "default_weight")]
    weight: u16,
    height: u32,
    #[serde(default = "default_valign")]
    valign: String,
    #[serde(default = "default_halign")]
    halign: String,
    #[serde(default = "default_line_spacing")]
    line_spacing: u32,
}

fn default_weight() -> u16 {
    400
}

fn default_valign() -> String {
    "center".to_string()
}

fn default_halign() -> String {
    "left".to_string()
}

fn default_line_spacing() -> u32 {
    120
}

async fn render_text(
    State(state): State<SharedState>,
    Json(body): Json<RenderTextRequest>,
) -> axum::response::Response {
    let state = state.read().await;

    match state.fonts.render_text(
        &body.text,
        &body.font,
        body.font_size,
        body.weight,
        body.height,
        &body.valign,
        &body.halign,
        body.line_spacing,
    ) {
        Some(bitmap) => {
            let png = bitmap.to_png();
            (
                [(axum::http::header::CONTENT_TYPE, "image/png")],
                png,
            )
                .into_response()
        }
        None => Json(json!({"error": "font not found"})).into_response(),
    }
}

async fn render_preview(
    State(_state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let _ = body;
    Json(json!({"ok": false, "error": "not implemented"}))
}
