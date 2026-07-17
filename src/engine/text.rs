//! FreeType-based text renderer for 1-bit monochrome output.
//!
//! Replaces the previous cosmic-text implementation after experiments showed that
//! FreeType's FT_RENDER_MODE_MONO with proper hinting produces superior 1-bit
//! output for thermal printers (Dymo LabelManager PnP at 180 DPI).
//!
//! Key advantages over cosmic-text/swash:
//! - True monochrome rasterization with dropout control (not alpha-threshold hack)
//! - Proper TrueType interpreter hinting (IV-35 style)
//! - Native bitmap font support via FT_Select_Size for OTB/BDF strikes
//! - Battle-tested quality (identical to Linux desktop rendering)
//!
//! See: https://github.com/dfrg/swash/issues/96
//!      https://github.com/googlefonts/fontations/pull/1496

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use freetype::face::LoadFlag;
use freetype::{Library, RenderMode};
use serde::Serialize;

use super::bitmap::Bitmap1Bit;

#[derive(Debug, Clone, Serialize)]
pub struct FontInfo {
    pub family: String,
    pub weights: Vec<u16>,
    pub has_italic: bool,
    /// Available pixel heights for bitmap fonts; empty for scalable fonts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub available_sizes: Vec<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FontGroups {
    pub favourites: Vec<FontInfo>,
    pub system: Vec<FontInfo>,
}

/// Internal record for a loaded font face.
struct FontEntry {
    path: PathBuf,
    face_index: isize,
    family: String,
    weight: u16,
    italic: bool,
    is_bitmap: bool,
    available_sizes: Vec<i32>,
}

pub struct FontStore {
    fonts: Vec<FontEntry>,
    groups: FontGroups,
}

impl FontStore {
    pub fn load(
        font_dir: &str,
        favourites: &[String],
        load_system_fonts: bool,
    ) -> Self {
        let library = Library::init().expect("Failed to initialize FreeType");

        let mut fonts = Vec::new();

        let otb_dir = Path::new(font_dir).join("otb");
        if otb_dir.is_dir() {
            scan_font_dir(&library, &otb_dir, &mut fonts);
        }

        if load_system_fonts {
            for dir in system_font_dirs() {
                let p = Path::new(&dir);
                if p.is_dir() {
                    scan_font_dir(&library, p, &mut fonts);
                }
            }
        }

        let mut family_weights: BTreeMap<String, Vec<u16>> = BTreeMap::new();
        let mut family_has_italic: BTreeMap<String, bool> = BTreeMap::new();
        let mut family_sizes: BTreeMap<String, Vec<i32>> = BTreeMap::new();
        for font in &fonts {
            family_weights
                .entry(font.family.clone())
                .or_default()
                .push(font.weight);
            if font.italic {
                family_has_italic.insert(font.family.clone(), true);
            }
            if font.is_bitmap && !font.available_sizes.is_empty() {
                let sizes = family_sizes.entry(font.family.clone()).or_default();
                for &s in &font.available_sizes {
                    if !sizes.contains(&s) {
                        sizes.push(s);
                    }
                }
            }
        }
        for weights in family_weights.values_mut() {
            weights.sort();
            weights.dedup();
        }
        for sizes in family_sizes.values_mut() {
            sizes.sort();
        }

        let mut favourites_list = Vec::new();
        for name in favourites {
            if let Some(weights) = family_weights.get(name.as_str()) {
                let mut available_sizes = family_sizes.get(name.as_str()).cloned().unwrap_or_default();
                available_sizes.dedup();
                favourites_list.push(FontInfo {
                    family: name.clone(),
                    weights: weights.clone(),
                    has_italic: family_has_italic.get(name.as_str()).copied().unwrap_or(false),
                    available_sizes,
                });
            }
        }

        let favourites_set: std::collections::BTreeSet<&str> =
            favourites.iter().map(|s| s.as_str()).collect();

        let mut system = Vec::new();
        if load_system_fonts {
            for (family, weights) in &family_weights {
                if !favourites_set.contains(family.as_str()) {
                    let mut available_sizes = family_sizes.get(family.as_str()).cloned().unwrap_or_default();
                    available_sizes.dedup();
                    system.push(FontInfo {
                        family: family.clone(),
                        weights: weights.clone(),
                        has_italic: family_has_italic.get(family.as_str()).copied().unwrap_or(false),
                        available_sizes,
                    });
                }
            }
        }

        let total = favourites_list.len() + system.len();
        tracing::debug!(
            "Font system ready: {} favourites, {} system ({} total families), {} font files",
            favourites_list.len(),
            system.len(),
            total,
            fonts.len(),
        );

        let groups = FontGroups {
            favourites: favourites_list,
            system,
        };

        Self {
            fonts,
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
        italic: bool,
        height: u32,
        valign: &str,
        halign: &str,
        line_spacing: u32,
    ) -> Option<Bitmap1Bit> {
        if text.is_empty() {
            return None;
        }

        let entry = self.find_font(font_family, weight, italic, font_size)?;
        let library = Library::init().ok()?;

        let mut face = library
            .new_face(&entry.path, entry.face_index)
            .ok()?;

        let load_flags = LoadFlag::TARGET_MONO;

        if entry.is_bitmap {
            let strike_idx = best_strike_index(&face, font_size as i32);
            let err = unsafe { freetype::ffi::FT_Select_Size(face.raw_mut(), strike_idx) };
            if err != freetype::ffi::FT_Err_Ok {
                tracing::warn!(
                    "FT_Select_Size failed for {} strike={}",
                    font_family,
                    strike_idx
                );
                return None;
            }
        } else if face.set_pixel_sizes(0, font_size).is_err() {
            tracing::warn!("set_pixel_sizes failed for {} size={}", font_family, font_size);
            return None;
        }

        let size_metrics = face.size_metrics();
        let ascent = size_metrics
            .map(|m| (m.ascender >> 6) as i32)
            .unwrap_or(font_size as i32);
        let descent = size_metrics
            .map(|m| ((-m.descender) >> 6) as i32)
            .unwrap_or(0);
        let font_height = size_metrics
            .map(|m| (m.height >> 6) as i32)
            .unwrap_or((ascent + descent) as i32);
        let line_step = (font_height as f32 * line_spacing as f32 / 100.0) as i32;

        let lines: Vec<&str> = text.split('\n').collect();

        let mut line_widths = Vec::new();
        let mut max_width: i32 = 0;
        for line in &lines {
            let w = measure_line(&face, line, load_flags);
            if w > max_width {
                max_width = w;
            }
            line_widths.push(w);
        }

        if max_width <= 0 {
            return None;
        }

        let width = max_width as u32;
        let ink_height = ascent + descent;
        let total_text_height = if lines.len() == 1 {
            ink_height
        } else {
            (lines.len() as i32 - 1) * line_step + ink_height
        };

        let first_baseline = match valign {
            "top" => ascent,
            "bottom" => height as i32 - total_text_height + ascent,
            _ => (height as i32 - total_text_height) / 2 + ascent,
        };

        let mut bmp = Bitmap1Bit::new(width, height);
        let mut baseline_y = first_baseline;

        for (line_idx, line) in lines.iter().enumerate() {
            let line_w = line_widths[line_idx];
            let x_offset = match halign {
                "center" => (width as i32 - line_w) / 2,
                "right" => width as i32 - line_w,
                _ => 0,
            };

            let mut pen_x: i32 = 0;

            for ch in line.chars() {
                let glyph_index = match face.get_char_index(ch as usize) {
                    Some(idx) => idx,
                    None => continue,
                };

                if face.load_glyph(glyph_index, load_flags).is_err() {
                    continue;
                }

                let glyph = face.glyph();
                if glyph.render_glyph(RenderMode::Mono).is_err() {
                    continue;
                }

                let bitmap = glyph.bitmap();
                let bmp_w = bitmap.width() as usize;
                let bmp_rows = bitmap.rows() as usize;
                let pitch = bitmap.pitch().unsigned_abs() as usize;
                let buffer = bitmap.buffer();
                let left = glyph.bitmap_left();
                let top = glyph.bitmap_top();

                for row in 0..bmp_rows {
                    for col in 0..bmp_w {
                        let byte_idx = row * pitch + col / 8;
                        let bit_pos = 7 - (col % 8);
                        if byte_idx >= buffer.len() {
                            continue;
                        }
                        if (buffer[byte_idx] >> bit_pos) & 1 == 0 {
                            continue;
                        }

                        let sx = x_offset + pen_x + left + col as i32;
                        let sy = baseline_y - top + row as i32;

                        if sx >= 0
                            && sy >= 0
                            && (sx as u32) < width
                            && (sy as u32) < height
                        {
                            bmp.set_pixel(sx as u32, sy as u32, true);
                        }
                    }
                }

                let advance = glyph.advance();
                pen_x += (advance.x >> 6) as i32;
            }

            baseline_y += line_step;
        }

        Some(bmp)
    }

    /// Find the best matching font entry for a family name, weight, and italic.
    fn find_font(&self, family: &str, weight: u16, italic: bool, target_size: u32) -> Option<&FontEntry> {
        let candidates: Vec<&FontEntry> = self
            .fonts
            .iter()
            .filter(|f| f.family == family && f.weight == weight && f.italic == italic)
            .collect();

        if !candidates.is_empty() {
            if let Some(entry) = candidates.iter().find(|f| {
                f.is_bitmap && f.available_sizes.contains(&(target_size as i32))
            }) {
                return Some(entry);
            }
            return Some(candidates[0]);
        }

        // Fallback: match family + italic, closest weight
        let family_style: Vec<&FontEntry> = self
            .fonts
            .iter()
            .filter(|f| f.family == family && f.italic == italic)
            .collect();
        if !family_style.is_empty() {
            return family_style
                .into_iter()
                .min_by_key(|f| (f.weight as i32 - weight as i32).unsigned_abs());
        }

        // Last resort: match family only, closest weight, prefer non-italic
        let family_fonts: Vec<&FontEntry> =
            self.fonts.iter().filter(|f| f.family == family).collect();
        if family_fonts.is_empty() {
            return None;
        }

        family_fonts
            .into_iter()
            .min_by_key(|f| {
                let weight_dist = (f.weight as i32 - weight as i32).unsigned_abs();
                let italic_penalty = if f.italic != italic { 1000u32 } else { 0 };
                weight_dist + italic_penalty
            })
    }
}

fn best_strike_index(face: &freetype::Face, target_height: i32) -> i32 {
    let raw = face.raw();
    let n = raw.num_fixed_sizes as usize;
    if n == 0 {
        return 0;
    }
    let mut best: i32 = 0;
    let mut best_h: i32 = 0;
    let mut smallest: i32 = 0;
    let mut smallest_h: i32 = i32::MAX;
    for i in 0..n {
        let h = unsafe {
            let s = *raw.available_sizes.add(i);
            let ppem = (s.y_ppem >> 6) as i32;
            if ppem >= 3 { ppem } else { s.height as i32 }
        };
        if h <= target_height && h > best_h {
            best = i as i32;
            best_h = h;
        }
        if h < smallest_h {
            smallest = i as i32;
            smallest_h = h;
        }
    }
    if best_h > 0 {
        best
    } else {
        smallest
    }
}

/// Measure the pixel width of a line of text.
fn measure_line(face: &freetype::Face, line: &str, load_flags: LoadFlag) -> i32 {
    let mut width: i32 = 0;
    for ch in line.chars() {
        let glyph_index = match face.get_char_index(ch as usize) {
            Some(idx) => idx,
            None => continue,
        };
        if face.load_glyph(glyph_index, load_flags).is_ok() {
            width += (face.glyph().advance().x >> 6) as i32;
        }
    }
    width
}

/// Scan a directory (recursively) for font files and extract metadata.
fn scan_font_dir(library: &Library, dir: &Path, fonts: &mut Vec<FontEntry>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            scan_font_dir(library, &path, fonts);
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !matches!(ext.as_str(), "ttf" | "otf" | "otb" | "ttc" | "bdf" | "pcf") {
            continue;
        }

        let num_faces = match library.new_face(&path, -1) {
            Ok(f) => f.raw().num_faces as isize,
            Err(_) => 1,
        };

        for face_idx in 0..num_faces {
            let face = match library.new_face(&path, face_idx) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let family = match face.family_name() {
                Some(name) => name,
                None => continue,
            };

            let style_flags = face.style_flags();
            let weight = if style_flags.contains(freetype::face::StyleFlag::BOLD) {
                700u16
            } else {
                400u16
            };
            let italic = style_flags.contains(freetype::face::StyleFlag::ITALIC)
                || face
                    .style_name()
                    .map(|s| {
                        let lower = s.to_lowercase();
                        lower.contains("italic") || lower.contains("oblique")
                    })
                    .unwrap_or(false);

            let is_bitmap = face.has_fixed_sizes() && !face.is_scalable();
            let available_sizes = if is_bitmap {
                let raw = face.raw();
                let n = raw.num_fixed_sizes as usize;
                (0..n)
                    .map(|i| unsafe {
                        let s = *raw.available_sizes.add(i);
                        let ppem = (s.y_ppem >> 6) as i32;
                        // fonttosfnt sometimes writes incorrect ppem for very small fonts
                        // (e.g. Tiny5 gets ppem=1). Fall back to height in that case.
                        if ppem >= 3 { ppem } else { s.height as i32 }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let already_exists = fonts.iter().any(|f| {
                f.family == family && f.weight == weight && f.italic == italic && f.path == path && f.face_index == face_idx
            });
            if already_exists {
                continue;
            }

            tracing::trace!(
                "Found font: {} weight={} italic={} bitmap={} sizes={:?} path={}",
                family,
                weight,
                italic,
                is_bitmap,
                available_sizes,
                path.display()
            );

            fonts.push(FontEntry {
                path: path.clone(),
                face_index: face_idx,
                family,
                weight,
                italic,
                is_bitmap,
                available_sizes,
            });
        }
    }
}

/// Platform-specific system font directories.
fn system_font_dirs() -> Vec<String> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "linux")]
    {
        dirs.push("/usr/share/fonts".to_string());
        dirs.push("/usr/local/share/fonts".to_string());
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(format!("{home}/.local/share/fonts"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push("/Library/Fonts".to_string());
        dirs.push("/System/Library/Fonts".to_string());
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(format!("{home}/Library/Fonts"));
        }
    }

    dirs
}
