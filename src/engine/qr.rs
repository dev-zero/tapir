use std::sync::LazyLock;

use datamatrix::SymbolSize as DmSize;
use qrcode2::{EcLevel, QrCode, Version};
use serde::Serialize;

use super::bitmap::Bitmap1Bit;

const QZ_STANDARD: i16 = 4;
const QZ_MICRO: i16 = 2;
const QZ_RMQR: i16 = 2;
const QZ_DATAMATRIX: i16 = 1;

const RMQR_VERSIONS: &[(i16, i16)] = &[
    (7, 43),
    (7, 59),
    (7, 77),
    (7, 99),
    (7, 139),
    (9, 43),
    (9, 59),
    (9, 77),
    (9, 99),
    (9, 139),
    (11, 27),
    (11, 43),
    (11, 59),
    (11, 77),
    (11, 99),
    (11, 139),
    (13, 27),
    (13, 43),
    (13, 59),
    (13, 77),
    (13, 99),
    (13, 139),
    (15, 43),
    (15, 59),
    (15, 77),
    (15, 99),
    (15, 139),
    (17, 43),
    (17, 59),
    (17, 77),
    (17, 99),
    (17, 139),
];

const EC_LEVELS: [EcLevel; 4] = [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H];

const DM_SIZES: &[(DmSize, i16, i16)] = &[
    (DmSize::Square10, 10, 10),
    (DmSize::Square12, 12, 12),
    (DmSize::Rect8x18, 8, 18),
    (DmSize::Square14, 14, 14),
    (DmSize::Rect8x32, 8, 32),
    (DmSize::Square16, 16, 16),
    (DmSize::Rect12x26, 12, 26),
    (DmSize::Square18, 18, 18),
    (DmSize::Rect8x48, 8, 48),
    (DmSize::Square20, 20, 20),
    (DmSize::Rect12x36, 12, 36),
    (DmSize::Rect8x64, 8, 64),
    (DmSize::Square22, 22, 22),
    (DmSize::Rect16x36, 16, 36),
    (DmSize::Rect8x80, 8, 80),
    (DmSize::Square24, 24, 24),
    (DmSize::Rect8x96, 8, 96),
    (DmSize::Rect12x64, 12, 64),
    (DmSize::Square26, 26, 26),
    (DmSize::Rect20x36, 20, 36),
    (DmSize::Rect16x48, 16, 48),
    (DmSize::Rect8x120, 8, 120),
    (DmSize::Rect20x44, 20, 44),
    (DmSize::Square32, 32, 32),
    (DmSize::Rect16x64, 16, 64),
    (DmSize::Rect8x144, 8, 144),
    (DmSize::Rect12x88, 12, 88),
    (DmSize::Rect26x40, 26, 40),
    (DmSize::Rect22x48, 22, 48),
    (DmSize::Rect24x48, 24, 48),
    (DmSize::Rect20x64, 20, 64),
    (DmSize::Square36, 36, 36),
    (DmSize::Rect24x64, 24, 64),
    (DmSize::Rect26x48, 26, 48),
    (DmSize::Rect26x64, 26, 64),
    (DmSize::Square40, 40, 40),
    (DmSize::Square44, 44, 44),
    (DmSize::Square48, 48, 48),
    (DmSize::Square52, 52, 52),
    (DmSize::Square64, 64, 64),
];

static DM_CAPACITIES: LazyLock<Vec<usize>> = LazyLock::new(|| {
    DM_SIZES
        .iter()
        .map(|(size, _, _)| {
            let mut lo = 0usize;
            let mut hi = 2048;
            while lo < hi {
                let mid = (lo + hi + 1) / 2;
                let data = vec![0u8; mid];
                if datamatrix::DataMatrix::encode(&data, *size).is_ok() {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            lo
        })
        .collect()
});

fn ec_from_str(s: &str) -> Option<EcLevel> {
    match s {
        "L" => Some(EcLevel::L),
        "M" => Some(EcLevel::M),
        "Q" => Some(EcLevel::Q),
        "H" => Some(EcLevel::H),
        _ => None,
    }
}

fn ec_to_str(ec: EcLevel) -> &'static str {
    match ec {
        EcLevel::L => "L",
        EcLevel::M => "M",
        EcLevel::Q => "Q",
        EcLevel::H => "H",
    }
}

fn quiet_zone(version: Version) -> i16 {
    match version {
        Version::Normal(_) => QZ_STANDARD,
        Version::Micro(_) => QZ_MICRO,
        Version::RectMicro(_, _) => QZ_RMQR,
    }
}

fn version_fits(version: Version, scale: u32, label_height: u32) -> bool {
    let qz = quiet_zone(version);
    let total = (version.height() + 2 * qz) as u32 * scale;
    total <= label_height
}

fn max_capacity_for_mode(version: Version, ec: EcLevel, test_char: u8) -> usize {
    let upper = match version {
        Version::Normal(_) => 4000,
        Version::Micro(_) => 50,
        Version::RectMicro(_, _) => 500,
    };
    let (mut lo, mut hi): (usize, usize) = (0, upper);
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let data: Vec<u8> = vec![test_char; mid];
        if QrCode::with_version(&data, version, ec).is_ok() {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

fn supported_ec_levels(version: Version) -> Vec<EcLevel> {
    EC_LEVELS
        .iter()
        .copied()
        .filter(|&ec| QrCode::with_version(b"1", version, ec).is_ok())
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct QrCapacity {
    pub n: usize,
    pub a: usize,
    pub b: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct QrVersionInfo {
    pub index: usize,
    pub label: String,
    pub modules_w: i16,
    pub modules_h: i16,
    pub quiet_zone: i16,
    pub ec_levels: Vec<String>,
    pub capacity: std::collections::BTreeMap<String, QrCapacity>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QrCapabilities {
    pub label_height: u32,
    pub standard: Vec<QrVersionInfo>,
    pub micro: Vec<QrVersionInfo>,
    pub rmqr: Vec<QrVersionInfo>,
    pub datamatrix: Vec<QrVersionInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QrRenderInfo {
    pub qr_type: String,
    pub version_index: usize,
    pub version_label: String,
    pub ec_level: String,
    pub pixel_scale: u32,
}

pub fn capabilities(label_height: u32) -> QrCapabilities {
    let mut standard = Vec::new();
    for v in 1..=40i16 {
        let version = Version::Normal(v);
        if !version_fits(version, 1, label_height) {
            break;
        }
        let ec_levels = supported_ec_levels(version);
        if ec_levels.is_empty() {
            continue;
        }
        let capacity = compute_capacity(version, &ec_levels);
        let mods = version.width();
        standard.push(QrVersionInfo {
            index: v as usize,
            label: format!("V{v}"),
            modules_w: mods,
            modules_h: mods,
            quiet_zone: QZ_STANDARD,
            ec_levels: ec_levels.iter().map(|e| ec_to_str(*e).to_string()).collect(),
            capacity,
        });
    }

    let mut micro = Vec::new();
    for v in 1..=4i16 {
        let version = Version::Micro(v);
        if !version_fits(version, 1, label_height) {
            break;
        }
        let ec_levels = supported_ec_levels(version);
        if ec_levels.is_empty() {
            continue;
        }
        let capacity = compute_capacity(version, &ec_levels);
        let mods = version.width();
        micro.push(QrVersionInfo {
            index: v as usize,
            label: format!("M{v}"),
            modules_w: mods,
            modules_h: mods,
            quiet_zone: QZ_MICRO,
            ec_levels: ec_levels.iter().map(|e| ec_to_str(*e).to_string()).collect(),
            capacity,
        });
    }

    let mut rmqr = Vec::new();
    for (idx, &(h, w)) in RMQR_VERSIONS.iter().enumerate() {
        let version = Version::RectMicro(h, w);
        if !version_fits(version, 1, label_height) {
            continue;
        }
        let ec_levels = supported_ec_levels(version);
        if ec_levels.is_empty() {
            continue;
        }
        let capacity = compute_capacity(version, &ec_levels);
        rmqr.push(QrVersionInfo {
            index: idx,
            label: format!("R{h}x{w}"),
            modules_w: w,
            modules_h: h,
            quiet_zone: QZ_RMQR,
            ec_levels: ec_levels.iter().map(|e| ec_to_str(*e).to_string()).collect(),
            capacity,
        });
    }

    let caps = &*DM_CAPACITIES;
    let mut datamatrix = Vec::new();
    for (idx, &(_, h, w)) in DM_SIZES.iter().enumerate() {
        let total_h = h as u32 + 2 * QZ_DATAMATRIX as u32;
        if total_h > label_height {
            continue;
        }
        let cap = caps[idx];
        let mut capacity = std::collections::BTreeMap::new();
        capacity.insert(
            "ECC200".to_string(),
            QrCapacity { n: cap, a: cap, b: cap },
        );
        datamatrix.push(QrVersionInfo {
            index: idx,
            label: format!("{h}x{w}"),
            modules_w: w,
            modules_h: h,
            quiet_zone: QZ_DATAMATRIX,
            ec_levels: vec!["ECC200".to_string()],
            capacity,
        });
    }

    QrCapabilities {
        label_height,
        standard,
        micro,
        rmqr,
        datamatrix,
    }
}

pub fn render_qr(
    data: &[u8],
    qr_type: &str,
    version_index: usize,
    ec_str: &str,
    pixel_scale: u32,
    label_height: u32,
) -> Result<(Bitmap1Bit, QrRenderInfo), RenderError> {
    if data.is_empty() {
        return Err(RenderError::EmptyData);
    }

    let scale = pixel_scale.clamp(1, 5);

    if qr_type == "datamatrix" {
        return render_dm_manual(data, version_index, scale, label_height);
    }

    let version = resolve_version(qr_type, version_index)?;
    let ec = ec_from_str(ec_str).ok_or(RenderError::InvalidEc)?;

    if !version_fits(version, scale, label_height) {
        return Err(RenderError::DoesNotFit);
    }

    let code = QrCode::with_version(data, version, ec).map_err(|_| {
        let max = max_fitting_prefix(data, version, ec);
        RenderError::DataTooLong { max_chars: max }
    })?;

    let bmp = render_code(&code, scale, label_height);
    let info = QrRenderInfo {
        qr_type: qr_type.to_string(),
        version_index,
        version_label: version_label(version),
        ec_level: ec_to_str(ec).to_string(),
        pixel_scale: scale,
    };

    Ok((bmp, info))
}

pub fn auto_render(
    data: &[u8],
    qr_type: &str,
    label_height: u32,
) -> Result<(Bitmap1Bit, QrRenderInfo), RenderError> {
    if data.is_empty() {
        return Err(RenderError::EmptyData);
    }

    if qr_type == "datamatrix" {
        return auto_render_dm(data, label_height);
    }

    let versions = all_versions_for_type(qr_type);
    if versions.is_empty() {
        return Err(RenderError::InvalidType);
    }

    // Score: higher is better. Prioritize EC > low version (coarse grid) > scale.
    let mut best: Option<(u32, Version, usize, EcLevel, u32)> = None;

    for (idx, &version) in versions.iter().enumerate() {
        let ec_levels = supported_ec_levels(version);
        for &ec in ec_levels.iter().rev() {
            if QrCode::with_version(data, version, ec).is_err() {
                continue;
            }
            for scale in (1..=5u32).rev() {
                if !version_fits(version, scale, label_height) {
                    continue;
                }
                let score = (ec as u32) * 100_000 + (500 - idx as u32) * 100 + scale;
                if best.is_none() || score > best.unwrap().0 {
                    best = Some((score, version, idx, ec, scale));
                }
                break; // first valid scale is the largest for this version
            }
        }
    }

    match best {
        Some((_, version, idx, ec, scale)) => {
            let code = QrCode::with_version(data, version, ec)
                .map_err(|_| RenderError::EncodeFailed)?;
            let bmp = render_code(&code, scale, label_height);
            let info = QrRenderInfo {
                qr_type: qr_type.to_string(),
                version_index: resolve_index(qr_type, idx),
                version_label: version_label(version),
                ec_level: ec_to_str(ec).to_string(),
                pixel_scale: scale,
            };
            Ok((bmp, info))
        }
        None => Err(RenderError::DataTooLong {
            max_chars: max_fitting_prefix_any(data, qr_type, label_height),
        }),
    }
}

pub fn validate(
    data: &[u8],
    qr_type: &str,
    version_index: usize,
    ec_str: &str,
) -> ValidateResult {
    if data.is_empty() {
        return ValidateResult {
            fits: true,
            max_chars: 0,
        };
    }

    if qr_type == "datamatrix" {
        let caps = &*DM_CAPACITIES;
        let cap = caps.get(version_index).copied().unwrap_or(0);
        return ValidateResult {
            fits: data.len() <= cap,
            max_chars: cap,
        };
    }

    let version = match resolve_version(qr_type, version_index) {
        Ok(v) => v,
        Err(_) => {
            return ValidateResult {
                fits: false,
                max_chars: 0,
            }
        }
    };
    let ec = match ec_from_str(ec_str) {
        Some(e) => e,
        None => {
            return ValidateResult {
                fits: false,
                max_chars: 0,
            }
        }
    };

    let fits = QrCode::with_version(data, version, ec).is_ok();
    let max_chars = if fits {
        data.len()
    } else {
        max_fitting_prefix(data, version, ec)
    };

    ValidateResult { fits, max_chars }
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidateResult {
    pub fits: bool,
    pub max_chars: usize,
}

#[derive(Debug)]
pub enum RenderError {
    EmptyData,
    InvalidType,
    InvalidVersion,
    InvalidEc,
    DoesNotFit,
    DataTooLong { max_chars: usize },
    EncodeFailed,
}

fn resolve_version(qr_type: &str, index: usize) -> Result<Version, RenderError> {
    match qr_type {
        "standard" => {
            if (1..=40).contains(&index) {
                Ok(Version::Normal(index as i16))
            } else {
                Err(RenderError::InvalidVersion)
            }
        }
        "micro" => {
            if (1..=4).contains(&index) {
                Ok(Version::Micro(index as i16))
            } else {
                Err(RenderError::InvalidVersion)
            }
        }
        "rmqr" => RMQR_VERSIONS
            .get(index)
            .map(|&(h, w)| Version::RectMicro(h, w))
            .ok_or(RenderError::InvalidVersion),
        _ => Err(RenderError::InvalidType),
    }
}

fn resolve_index(qr_type: &str, internal_idx: usize) -> usize {
    match qr_type {
        "standard" | "micro" => internal_idx + 1,
        _ => internal_idx,
    }
}

fn all_versions_for_type(qr_type: &str) -> Vec<Version> {
    match qr_type {
        "standard" => (1..=40).map(Version::Normal).collect(),
        "micro" => (1..=4).map(Version::Micro).collect(),
        "rmqr" => RMQR_VERSIONS
            .iter()
            .map(|&(h, w)| Version::RectMicro(h, w))
            .collect(),
        _ => Vec::new(),
    }
}

fn version_label(version: Version) -> String {
    match version {
        Version::Normal(v) => format!("V{v}"),
        Version::Micro(v) => format!("M{v}"),
        Version::RectMicro(h, w) => format!("R{h}x{w}"),
    }
}

fn compute_capacity(
    version: Version,
    ec_levels: &[EcLevel],
) -> std::collections::BTreeMap<String, QrCapacity> {
    let mut map = std::collections::BTreeMap::new();
    for &ec in ec_levels {
        map.insert(
            ec_to_str(ec).to_string(),
            QrCapacity {
                n: max_capacity_for_mode(version, ec, b'0'),
                a: max_capacity_for_mode(version, ec, b'A'),
                b: max_capacity_for_mode(version, ec, b'a'),
            },
        );
    }
    map
}

fn render_code(code: &QrCode, scale: u32, label_height: u32) -> Bitmap1Bit {
    let mod_w = code.width() as i32;
    let mod_h = code.height() as i32;
    let version = code.version();
    let qz = quiet_zone(version) as i32;

    let block_w = (mod_w + 2 * qz) as u32 * scale;
    let block_h = (mod_h + 2 * qz) as u32 * scale;

    let y_offset = (label_height.saturating_sub(block_h)) / 2;

    let mut bmp = Bitmap1Bit::new(block_w, label_height);

    for my in 0..mod_h {
        for mx in 0..mod_w {
            if code[(mx as usize, my as usize)] == qrcode2::Color::Dark {
                let px = (qz + mx) as u32 * scale;
                let py = y_offset + (qz + my) as u32 * scale;
                for dy in 0..scale {
                    for dx in 0..scale {
                        bmp.set_pixel(px + dx, py + dy, true);
                    }
                }
            }
        }
    }

    bmp
}

fn max_fitting_prefix(data: &[u8], version: Version, ec: EcLevel) -> usize {
    if data.is_empty() {
        return 0;
    }
    let (mut lo, mut hi): (usize, usize) = (0, data.len());
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if QrCode::with_version(&data[..mid], version, ec).is_ok() {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

fn max_fitting_prefix_any(data: &[u8], qr_type: &str, label_height: u32) -> usize {
    if qr_type == "datamatrix" {
        let caps = &*DM_CAPACITIES;
        let mut best = 0usize;
        for (idx, &(_, h, _)) in DM_SIZES.iter().enumerate() {
            let total_h = h as u32 + 2 * QZ_DATAMATRIX as u32;
            if total_h > label_height {
                continue;
            }
            let cap = caps[idx];
            if cap > best {
                best = cap;
            }
        }
        return best.min(data.len());
    }
    let versions = all_versions_for_type(qr_type);
    let mut best = 0usize;
    for &version in versions.iter().rev() {
        if !version_fits(version, 1, label_height) {
            continue;
        }
        for &ec in &[EcLevel::L, EcLevel::M] {
            let n = max_fitting_prefix(data, version, ec);
            if n > best {
                best = n;
            }
            if best >= data.len() {
                return best;
            }
        }
    }
    best
}

fn render_dm_manual(
    data: &[u8],
    size_index: usize,
    scale: u32,
    label_height: u32,
) -> Result<(Bitmap1Bit, QrRenderInfo), RenderError> {
    let &(dm_size, h, w) = DM_SIZES.get(size_index).ok_or(RenderError::InvalidVersion)?;

    let total_h = (h as u32 + 2 * QZ_DATAMATRIX as u32) * scale;
    if total_h > label_height {
        return Err(RenderError::DoesNotFit);
    }

    let code = datamatrix::DataMatrix::encode(data, dm_size).map_err(|_| {
        let caps = &*DM_CAPACITIES;
        let cap = caps.get(size_index).copied().unwrap_or(0);
        RenderError::DataTooLong { max_chars: cap }
    })?;

    let bmp = render_dm_bitmap(&code, scale, label_height);
    let info = QrRenderInfo {
        qr_type: "datamatrix".to_string(),
        version_index: size_index,
        version_label: format!("{h}x{w}"),
        ec_level: "ECC200".to_string(),
        pixel_scale: scale,
    };

    Ok((bmp, info))
}

fn auto_render_dm(
    data: &[u8],
    label_height: u32,
) -> Result<(Bitmap1Bit, QrRenderInfo), RenderError> {
    let caps = &*DM_CAPACITIES;
    // DM has only ECC200, so priority is: lowest version (coarse) > largest scale.
    let mut best: Option<(u32, usize, u32)> = None;

    for (idx, &(_, h, _)) in DM_SIZES.iter().enumerate() {
        let cap = caps[idx];
        if data.len() > cap {
            continue;
        }
        let total_h = h as u32 + 2 * QZ_DATAMATRIX as u32;
        for scale in (1..=5u32).rev() {
            if total_h * scale > label_height {
                continue;
            }
            let score = (500 - idx as u32) * 100 + scale;
            if best.is_none() || score > best.unwrap().0 {
                best = Some((score, idx, scale));
            }
            break;
        }
    }

    match best {
        Some((_, idx, scale)) => {
            let (dm_size, h, w) = DM_SIZES[idx];
            let code = datamatrix::DataMatrix::encode(data, dm_size)
                .map_err(|_| RenderError::EncodeFailed)?;
            let bmp = render_dm_bitmap(&code, scale, label_height);
            let info = QrRenderInfo {
                qr_type: "datamatrix".to_string(),
                version_index: idx,
                version_label: format!("{h}x{w}"),
                ec_level: "ECC200".to_string(),
                pixel_scale: scale,
            };
            Ok((bmp, info))
        }
        None => {
            let max = max_fitting_prefix_any(data, "datamatrix", label_height);
            Err(RenderError::DataTooLong { max_chars: max })
        }
    }
}

fn render_dm_bitmap(code: &datamatrix::DataMatrix, scale: u32, label_height: u32) -> Bitmap1Bit {
    let bitmap = code.bitmap();
    let mod_w = bitmap.width() as u32;
    let mod_h = bitmap.height() as u32;
    let qz = QZ_DATAMATRIX as u32;

    let block_w = (mod_w + 2 * qz) * scale;
    let block_h = (mod_h + 2 * qz) * scale;
    let y_offset = label_height.saturating_sub(block_h) / 2;

    let mut bmp = Bitmap1Bit::new(block_w, label_height);

    for (x, y) in bitmap.pixels() {
        let px = (qz + x as u32) * scale;
        let py = y_offset + (qz + y as u32) * scale;
        for dy in 0..scale {
            for dx in 0..scale {
                bmp.set_pixel(px + dx, py + dy, true);
            }
        }
    }

    bmp
}
