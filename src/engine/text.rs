use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Mutex;

use cosmic_text::{
    Align, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SubpixelBin, SwashCache, Weight,
};
use serde::Serialize;

use super::bitmap::Bitmap1Bit;

#[derive(Debug, Clone, Serialize)]
pub struct FontInfo {
    pub family: String,
    pub weights: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FontGroups {
    pub medium: Vec<FontInfo>,
    pub small: Vec<FontInfo>,
    pub system: Vec<FontInfo>,
}

pub struct FontStore {
    font_system: Mutex<FontSystem>,
    swash_cache: Mutex<SwashCache>,
    groups: FontGroups,
}

impl FontStore {
    pub fn load(
        bundled_dir: &str,
        favourites_medium: &[String],
        favourites_small: &[String],
        show_all_fonts: bool,
    ) -> Self {
        let mut db = cosmic_text::fontdb::Database::new();

        let path = Path::new(bundled_dir);
        if path.is_dir() {
            db.load_fonts_dir(path);
        }

        db.load_system_fonts();

        let mut family_weights: BTreeMap<String, BTreeSet<u16>> = BTreeMap::new();
        for face in db.faces() {
            for (name, _) in &face.families {
                family_weights
                    .entry(name.clone())
                    .or_default()
                    .insert(face.weight.0);
            }
        }

        let mut medium = Vec::new();
        for name in favourites_medium {
            if let Some(weights) = family_weights.get(name.as_str()) {
                medium.push(FontInfo {
                    family: name.clone(),
                    weights: weights.iter().copied().collect(),
                });
            }
        }

        let mut small = Vec::new();
        for name in favourites_small {
            if let Some(weights) = family_weights.get(name.as_str()) {
                small.push(FontInfo {
                    family: name.clone(),
                    weights: weights.iter().copied().collect(),
                });
            }
        }

        let medium_set: BTreeSet<&str> = favourites_medium.iter().map(|s| s.as_str()).collect();
        let small_set: BTreeSet<&str> = favourites_small.iter().map(|s| s.as_str()).collect();

        let mut system = Vec::new();
        if show_all_fonts {
            for (family, weights) in &family_weights {
                if !medium_set.contains(family.as_str())
                    && !small_set.contains(family.as_str())
                {
                    system.push(FontInfo {
                        family: family.clone(),
                        weights: weights.iter().copied().collect(),
                    });
                }
            }
        }

        let total = medium.len() + small.len() + system.len();
        let font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);

        tracing::debug!(
            "Font system ready: {} medium, {} small, {} system ({} total families)",
            medium.len(),
            small.len(),
            system.len(),
            total,
        );

        let groups = FontGroups {
            medium,
            small,
            system,
        };

        Self {
            font_system: Mutex::new(font_system),
            swash_cache: Mutex::new(SwashCache::new()),
            groups,
        }
    }

    pub fn groups(&self) -> &FontGroups {
        &self.groups
    }

    pub fn render_text(
        &self,
        text: &str,
        font_family: &str,
        font_size: u32,
        weight: u16,
        height: u32,
        valign: &str,
        halign: &str,
        line_spacing: u32,
    ) -> Option<Bitmap1Bit> {
        if text.is_empty() {
            return None;
        }

        let mut font_system = self.font_system.lock().unwrap();
        let mut swash_cache = self.swash_cache.lock().unwrap();

        let font_size_f = font_size as f32;
        let line_height = font_size_f * (line_spacing as f32 / 100.0);

        let mut buffer = Buffer::new(&mut font_system, Metrics::new(font_size_f, line_height));
        buffer.set_size(&mut font_system, None, None);

        let h_align = match halign {
            "center" => Some(Align::Center),
            "right" => Some(Align::Right),
            _ => Some(Align::Left),
        };

        let attrs = Attrs::new()
            .family(Family::Name(font_family))
            .weight(Weight(weight));
        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);

        for line in buffer.lines.iter_mut() {
            line.set_align(h_align);
        }
        buffer.shape_until_scroll(&mut font_system, false);

        let mut max_x: i32 = 0;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let end_x = (glyph.x + glyph.w).ceil() as i32;
                if end_x > max_x {
                    max_x = end_x;
                }
            }
        }

        if max_x <= 0 {
            return None;
        }

        let width = max_x as u32;

        buffer.set_size(&mut font_system, Some(width as f32), None);
        buffer.shape_until_scroll(&mut font_system, false);

        let mut min_y: i32 = i32::MAX;
        let mut max_y: i32 = i32::MIN;
        let mut temp_pixels: Vec<(i32, i32)> = Vec::new();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let mut physical = glyph.physical((0.0, run.line_y), 1.0);
                physical.cache_key.x_bin = SubpixelBin::Zero;
                physical.cache_key.y_bin = SubpixelBin::Zero;
                swash_cache.with_pixels(
                    &mut font_system,
                    physical.cache_key,
                    cosmic_text::Color::rgb(0, 0, 0),
                    |dx, dy, color| {
                        if color.a() > 127 {
                            let px = physical.x + dx;
                            let py = physical.y + dy;
                            temp_pixels.push((px, py));
                            if py < min_y {
                                min_y = py;
                            }
                            if py > max_y {
                                max_y = py;
                            }
                        }
                    },
                );
            }
        }

        if temp_pixels.is_empty() {
            return None;
        }

        let rendered_height = (max_y - min_y + 1) as u32;
        let y_offset = match valign {
            "top" => -min_y,
            "bottom" => height as i32 - max_y - 1,
            _ => (height as i32 - rendered_height as i32) / 2 - min_y,
        };

        let mut bmp = Bitmap1Bit::new(width, height);
        for (px, py) in &temp_pixels {
            let final_y = py + y_offset;
            if *px >= 0 && final_y >= 0 && (*px as u32) < width && (final_y as u32) < height {
                bmp.set_pixel(*px as u32, final_y as u32, true);
            }
        }

        Some(bmp)
    }
}
