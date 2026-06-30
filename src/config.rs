use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub default_label: Option<String>,
    #[serde(default = "default_canvas_width")]
    pub default_canvas_width: u16,
    #[serde(default)]
    pub font_favourites: Vec<String>,
    #[serde(default = "default_load_system_fonts")]
    pub load_system_fonts: bool,
}

fn default_canvas_width() -> u16 {
    200
}

fn default_load_system_fonts() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDef {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    #[serde(default)]
    pub product_id_storage: Option<u16>,
    pub max_tape_mm: u8,
    #[serde(default = "default_dpi")]
    pub dpi: u16,
    /// Lines between ESC-A sync commands (flow control interval).
    #[serde(default = "default_synwait")]
    pub synwait: u16,
    #[serde(default = "default_feed_lines_for_cut")]
    pub feed_lines_for_cut: u16,
    #[serde(default = "default_minimal_autofeed_lines")]
    pub minimal_autofeed_lines: u16,
}

fn default_dpi() -> u16 {
    180
}

fn default_synwait() -> u16 {
    64
}

fn default_minimal_autofeed_lines() -> u16 {
    180
}

fn default_feed_lines_for_cut() -> u16 {
    57
}

impl AppConfig {
    pub fn load_or_default(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("Failed to parse {path}: {e}, using defaults");
                    Self::default()
                }
            },
            Err(_) => {
                tracing::info!("No {path} found, writing defaults");
                let config = Self::default();
                if let Err(e) = config.save(path) {
                    tracing::warn!("Could not write default config: {e}");
                }
                config
            }
        }
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_label: None,
            default_canvas_width: default_canvas_width(),
            font_favourites: Vec::new(),
            load_system_fonts: default_load_system_fonts(),
        }
    }
}

pub fn load_devices(dir: &str) -> Vec<DeviceDef> {
    let mut devices = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            tracing::info!("No devices directory at {dir}, writing default");
            if std::fs::create_dir_all(dir).is_ok() {
                let default = default_device();
                let path = format!("{dir}/labelmanager-pnp.toml");
                if let Ok(contents) = toml::to_string_pretty(&default) {
                    let _ = std::fs::write(&path, contents);
                }
                devices.push(default);
            }
            return devices;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str::<DeviceDef>(&contents) {
                    Ok(dev) => {
                        tracing::debug!("Loaded device: {} (0x{:04x})", dev.name, dev.product_id);
                        devices.push(dev);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {e}", path.display());
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read {}: {e}", path.display());
                }
            }
        }
    }

    if devices.is_empty() {
        tracing::info!("No device files found in {dir}, using default");
        devices.push(default_device());
    }

    devices.sort_by(|a, b| a.name.cmp(&b.name));
    devices
}

fn default_device() -> DeviceDef {
    DeviceDef {
        name: "LabelManager PnP".to_string(),
        vendor_id: 0x0922,
        product_id: 0x1002,
        product_id_storage: Some(0x1001),
        max_tape_mm: 12,
        dpi: 180,
        synwait: 64,
        feed_lines_for_cut: 57,
        minimal_autofeed_lines: 180,
    }
}
