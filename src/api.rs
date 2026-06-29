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
use crate::usb::{device, protocol::PrintJob};

type SharedState = Arc<RwLock<AppState>>;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/status", get(status))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/labels", get(list_labels))
        .route("/labels/reload", post(reload_labels))
        .route("/fonts", get(list_fonts))
        .route("/render-text", post(render_text))
        .route("/qr-capabilities", get(qr_capabilities))
        .route("/render-qr", post(render_qr))
        .route("/qr-validate", post(qr_validate))
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

async fn put_settings(
    State(_state): State<SharedState>,
    Json(_body): Json<Value>,
) -> Json<Value> {
    Json(json!({"ok": true}))
}

async fn list_labels(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;

    let labels: Vec<Value> = state
        .labels
        .iter()
        .map(|l| {
            json!({
                "name": l.name,
                "tape_width_mm": l.tape_width_mm,
                "background_color": l.background_color,
                "foreground_color": l.foreground_color,
                "tape_type": l.tape_type,
                "dymo_tape_color_id": l.dymo_tape_color_id,
                "height_px": l.height_px(),
                "margin_px": l.margin_px(),
            })
        })
        .collect();

    Json(json!(labels))
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
    #[serde(default = "default_auto_feed")]
    auto_feed: String,
}

fn default_auto_feed() -> String {
    "symmetric".to_string()
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

    let (bytes_per_line, tape_color_id, synwait, feed_lines_for_cut, minimal_autofeed_lines) = match dev_def {
        Some(def) => {
            let bpl = ((bitmap.height + 7) / 8) as u8;
            (bpl, 0u8, def.synwait, def.feed_lines_for_cut, def.minimal_autofeed_lines)
        }
        None => (8u8, 0u8, 64u16, 57u16, 180u16),
    };

    let job = PrintJob {
        bytes_per_line,
        tape_color_id,
        synwait,
    };

    let mut encoded = job.encode(&bitmap);

    let feed_lines = match params.auto_feed.as_str() {
        "none" => 0,
        "cutter" => feed_lines_for_cut,
        _ => {
            let image_lines = bitmap.width as u16;
            let min_feed = 2 * feed_lines_for_cut;
            let feed_for_minimum = minimal_autofeed_lines.saturating_sub(image_lines);
            min_feed.max(feed_for_minimum)
        }
    };

    if feed_lines > 0 {
        encoded.extend_from_slice(&job.encode_feed(feed_lines));
    }

    match device::send_print_data(product_id, &encoded) {
        Ok(()) => Json(json!({"ok": true, "bytes_sent": encoded.len(), "auto_feed": params.auto_feed, "feed_lines": feed_lines})),
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

    let (bytes_per_line, synwait, feed_lines_for_cut) = match dev_def {
        Some(def) => {
            let bpl = (8u8 * def.max_tape_mm) / 12;
            (bpl, def.synwait, def.feed_lines_for_cut)
        }
        None => (8u8, 64u16, 57u16),
    };

    let job = PrintJob {
        bytes_per_line,
        tape_color_id: 0,
        synwait,
    };

    let lines = feed_lines_for_cut;
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
    #[serde(default)]
    italic: bool,
    height: u32,
    #[serde(default = "default_valign")]
    valign: String,
    #[serde(default = "default_halign")]
    halign: String,
    #[serde(default = "default_line_spacing")]
    line_spacing: u32,
    #[serde(default = "default_pixel_scale")]
    pixel_scale: u32,
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

fn default_pixel_scale() -> u32 {
    1
}

async fn render_text(
    State(state): State<SharedState>,
    Json(body): Json<RenderTextRequest>,
) -> axum::response::Response {
    let state = state.read().await;
    let scale = body.pixel_scale.clamp(1, 5);
    let effective_height = (body.height + scale - 1) / scale;

    match state.fonts.render_text(
        &body.text,
        &body.font,
        body.font_size,
        body.weight,
        body.italic,
        effective_height,
        &body.valign,
        &body.halign,
        body.line_spacing,
    ) {
        Some(bitmap) => {
            let bitmap = if scale > 1 {
                bitmap.scale_nearest(scale).crop_height(body.height)
            } else {
                bitmap
            };
            let png = bitmap.to_png();
            (
                [(axum::http::header::CONTENT_TYPE, "image/png")],
                png,
            )
                .into_response()
        }
        None => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": "render failed"})),
        )
            .into_response(),
    }
}

async fn render_preview(
    State(_state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let _ = body;
    Json(json!({"ok": false, "error": "not implemented"}))
}

#[derive(Deserialize)]
struct QrCapabilitiesParams {
    height: u32,
}

async fn qr_capabilities(Query(params): Query<QrCapabilitiesParams>) -> Json<Value> {
    let caps = crate::engine::qr::capabilities(params.height);
    Json(json!(caps))
}

#[derive(Deserialize)]
struct RenderQrRequest {
    data: String,
    height: u32,
    qr_type: String,
    #[serde(default)]
    auto: bool,
    #[serde(default = "default_qr_version")]
    version: usize,
    #[serde(default = "default_halign")]
    ec_level: String,
    #[serde(default = "default_pixel_scale")]
    pixel_scale: u32,
}

fn default_qr_version() -> usize {
    1
}

async fn render_qr(Json(body): Json<RenderQrRequest>) -> axum::response::Response {
    let result = if body.auto {
        crate::engine::qr::auto_render(body.data.as_bytes(), &body.qr_type, body.height)
    } else {
        crate::engine::qr::render_qr(
            body.data.as_bytes(),
            &body.qr_type,
            body.version,
            &body.ec_level,
            body.pixel_scale,
            body.height,
        )
    };

    match result {
        Ok((bitmap, info)) => {
            let png = bitmap.to_png();
            (
                [
                    (axum::http::header::CONTENT_TYPE, "image/png".to_string()),
                    (
                        axum::http::HeaderName::from_static("x-qr-config"),
                        serde_json::to_string(&info).unwrap_or_default(),
                    ),
                ],
                png,
            )
                .into_response()
        }
        Err(crate::engine::qr::RenderError::DataTooLong { max_chars }) => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": "data_too_long", "max_chars": max_chars})),
        )
            .into_response(),
        Err(crate::engine::qr::RenderError::EmptyData) => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": "empty_data"})),
        )
            .into_response(),
        Err(crate::engine::qr::RenderError::DoesNotFit) => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": "does_not_fit"})),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("{e:?}")})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct QrValidateRequest {
    data: String,
    qr_type: String,
    version: usize,
    ec_level: String,
}

async fn qr_validate(Json(body): Json<QrValidateRequest>) -> Json<Value> {
    let result = crate::engine::qr::validate(
        body.data.as_bytes(),
        &body.qr_type,
        body.version,
        &body.ec_level,
    );
    Json(json!(result))
}
