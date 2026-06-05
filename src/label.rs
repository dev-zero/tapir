use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelDef {
    pub name: String,
    pub tape_width_mm: u8,
    #[serde(default = "default_bg")]
    pub background_color: String,
    #[serde(default = "default_fg")]
    pub foreground_color: String,
    #[serde(default)]
    pub tape_type: Option<String>,
    #[serde(default)]
    pub dymo_tape_color_id: u8,
}

fn default_bg() -> String {
    "#FFFFFF".to_string()
}

fn default_fg() -> String {
    "#000000".to_string()
}

impl LabelDef {
    /// Printable height in pixels for this tape width.
    pub fn height_px(&self) -> u32 {
        let bytes_per_line = (8 * self.tape_width_mm as u32) / 12;
        bytes_per_line * 8
    }
}

pub fn load_labels(dir: &str) -> Vec<LabelDef> {
    let mut labels = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            tracing::info!("No labels directory at {dir}");
            return labels;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str::<LabelDef>(&contents) {
                    Ok(label) => {
                        tracing::debug!("Loaded label: {} ({}mm)", label.name, label.tape_width_mm);
                        labels.push(label);
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

    labels.sort_by(|a, b| a.name.cmp(&b.name));
    labels
}
