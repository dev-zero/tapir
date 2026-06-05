use std::sync::Arc;
use tokio::sync::RwLock;

use axum::{
    Json, Router,
    extract::State,
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
        .route("/printers", get(list_printers))
        .route("/print", post(print_bitmap))
        .route("/preview", post(render_preview))
}

async fn status(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    let devices = device::enumerate_devices();
    let connected = devices.iter().find(|d| !d.needs_modeswitch);

    match connected {
        Some(dev) => {
            let tape_mm = state
                .config
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
        "devices": state.config.devices,
    }))
}

#[derive(Deserialize)]
struct SettingsUpdate {
    #[serde(default)]
    devices: Option<Vec<crate::config::DeviceDef>>,
}

async fn put_settings(
    State(state): State<SharedState>,
    Json(body): Json<SettingsUpdate>,
) -> Json<Value> {
    let mut state = state.write().await;
    if let Some(devices) = body.devices {
        state.config.devices = devices;
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

async fn print_bitmap(
    State(state): State<SharedState>,
    body: axum::body::Bytes,
) -> Json<Value> {
    let bitmap = match Bitmap1Bit::from_png(&body) {
        Ok(b) => b,
        Err(e) => return Json(json!({"ok": false, "error": format!("PNG decode failed: {e}")})),
    };

    let state = state.read().await;
    let devices = device::enumerate_devices();

    let printer = match devices.iter().find(|d| !d.needs_modeswitch) {
        Some(d) => d,
        None => {
            let needs_switch = devices.iter().find(|d| d.needs_modeswitch);
            if let Some(storage_dev) = needs_switch {
                let dev_def = state
                    .config
                    .devices
                    .iter()
                    .find(|d| d.product_id_storage == Some(storage_dev.product_id));

                if let Some(def) = dev_def {
                    if let Err(e) = device::modeswitch(storage_dev.product_id, &def.modeswitch_payload) {
                        return Json(json!({"ok": false, "error": format!("Modeswitch failed: {e}")}));
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    return Json(json!({"ok": false, "error": "Device switched to printer mode, retry print"}));
                }
            }
            return Json(json!({"ok": false, "error": "No printer connected"}));
        }
    };

    let dev_def = state
        .config
        .devices
        .iter()
        .find(|d| d.product_id == printer.product_id)
        .cloned();

    let (bytes_per_line, tape_color_id, synwait, product_id) = match dev_def {
        Some(def) => {
            let bpl = (8u8 * def.max_tape_mm) / 12;
            (bpl, 0u8, def.synwait, def.product_id)
        }
        None => (8u8, 0u8, 64u16, printer.product_id),
    };

    let job = PrintJob {
        bytes_per_line,
        tape_color_id,
        synwait,
    };

    let encoded = job.encode(&bitmap);

    match device::send_print_data(product_id, &encoded) {
        Ok(()) => Json(json!({"ok": true, "bytes_sent": encoded.len()})),
        Err(e) => Json(json!({"ok": false, "error": format!("Print failed: {e}")})),
    }
}

async fn render_preview(
    State(_state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let _ = body;
    Json(json!({"ok": false, "error": "not implemented"}))
}
