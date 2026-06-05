use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub devices: Vec<DeviceDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDef {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    #[serde(default)]
    pub product_id_storage: Option<u16>,
    #[serde(default = "default_interface_class")]
    pub interface_class: u8,
    pub max_tape_mm: u8,
    #[serde(default = "default_dpi")]
    pub dpi: u16,
    #[serde(default = "default_synwait")]
    pub synwait: u16,
    #[serde(default)]
    pub modeswitch_payload: Vec<u8>,
}

fn default_interface_class() -> u8 {
    0x07
}

fn default_dpi() -> u16 {
    180
}

fn default_synwait() -> u16 {
    64
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
                tracing::info!("No {path} found, using defaults");
                Self::default()
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
            devices: vec![DeviceDef {
                name: "LabelManager PnP".to_string(),
                vendor_id: 0x0922,
                product_id: 0x1002,
                product_id_storage: Some(0x1001),
                interface_class: 0x07,
                max_tape_mm: 12,
                dpi: 180,
                synwait: 64,
                modeswitch_payload: vec![0x1B, 0x5A, 0x01],
            }],
        }
    }
}
