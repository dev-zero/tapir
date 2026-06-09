# LabelManager PnP — Rust Native Web Interface

## Motivation

Existing solutions (labelle, labelle-web) are Python-based and depend on libusb.
Text rendering at these resolutions (64px for 12mm tape) is fragile — text centering
and clipping are known pain points. A pixel editor gives direct control over
what lands on the tape, bypassing font-rendering issues entirely.

This project is a **Rust-native** label printer interface for the Dymo LabelManager PnP,
running on any Linux box, accessible via browser.

---

## Hardware Facts

| Parameter | Value |
|-----------|-------|
| Vendor ID | `0x0922` |
| Product ID (mass storage) | `0x1001` |
| Product ID (printer mode) | `0x1002` |
| Mode switch message | `0x1b 0x5a 0x01` (ESC 'Z' 0x01) |
| DPI | 180 |
| Pixels per mm | ~7.09 |
| ESC byte | `0x1B` |
| SYN byte (print data prefix) | `0x16` |

### Tape Sizes

| Tape (mm) | Printable height (px) | Bytes per line |
|-----------|-----------------------|----------------|
| 6 | 32 | 4 |
| 9 | 48 | 6 |
| 12 | 64 | 8 |
| 19 | ~104 (extrapolated) | 13 |

Formula: `max_bytes_per_line = 8 * tape_size_mm / 12`, `height_px = max_bytes_per_line * 8`

### Protocol (ESC command set, subset of LabelWriter 450)

| Command | Bytes | Description |
|---------|-------|-------------|
| Status | `1B 41` | Request status, read 512-byte response |
| Dot Tab | `1B 42 <n>` | Left margin offset (bytes, n*8 pixels) |
| Tape Color | `1B 43 <n>` | Heat sensitivity for tape type |
| Bytes/Line | `1B 44 <n>` | Declare width of subsequent SYN data |
| Cut | `1B 45` | Trigger cutter |
| Print line | `16 <data...>` | SYN + n bytes of bitmap row |
| Skip lines | `16` (repeated) | SYN with bytes_per_line=0 → blank lines |

### Print Flow
1. Open USB device (vendor `0x0922`, product `0x1002`)
2. Claim printer interface (class `0x07`) or HID interface (class `0x03`)
3. Detach kernel driver if active
4. Find IN/OUT bulk endpoints
5. Set tape color: `ESC C 0`
6. For each **column** of the bitmap (image is rotated 270°):
   - Set bytes_per_line: `ESC D <n>`
   - Send: `SYN <row_bytes>`
7. Request status: `ESC A`
8. Read response
9. Flow control: every ~64 SYN lines, pause and poll status

### Key Insight (from labelle source)
The bitmap is **rotated 270°** before sending. The image's X-axis becomes the tape's
feed direction, and Y-axis becomes the tape height. Each "line" sent to the printer
is a vertical slice (column of the original image), packed MSB-first into bytes.

---

## Device Definitions

Devices are identified by USB Vendor:Product ID pairs. Each device definition
lives in a config struct (or the global `config.toml`), pinned to its IDs.

```toml
# config.toml — [devices] section

[[devices]]
name = "LabelManager PnP"
vendor_id = 0x0922
product_id = 0x1002              # Printer mode (post-modeswitch)
product_id_storage = 0x1001      # Mass storage mode (pre-modeswitch)
interface_class = 0x07           # Printer class (fallback: 0x03 HID)
max_tape_mm = 12
dpi = 180
print_head_height_mm = 8.2
head_to_cutter_mm = 8.1
synwait = 64                     # Flow control: lines before status poll
modeswitch_payload = [0x1B, 0x5A, 0x01]

[[devices]]
name = "LabelManager Wireless PnP"
vendor_id = 0x0922
product_id = 0x1008
product_id_storage = 0x1007
interface_class = 0x07
max_tape_mm = 12
dpi = 180
print_head_height_mm = 8.2
head_to_cutter_mm = 8.1
synwait = 64
modeswitch_payload = [0x1B, 0x5A, 0x01]

[[devices]]
name = "LabelManager 280"
vendor_id = 0x0922
product_id = 0x1006
product_id_storage = 0x1005
interface_class = 0x07
max_tape_mm = 12
dpi = 180
print_head_height_mm = 8.2
head_to_cutter_mm = 8.1
synwait = 64
modeswitch_payload = [0x1B, 0x5A, 0x01]

[[devices]]
name = "LabelManager 420P"
vendor_id = 0x0922
product_id = 0x1004
product_id_storage = 0x1003
interface_class = 0x07
max_tape_mm = 19
dpi = 180
print_head_height_mm = 8.2
head_to_cutter_mm = 8.1
synwait = 64
modeswitch_payload = [0x1B, 0x5A, 0x01]

[[devices]]
name = "LabelMANAGER PC"
vendor_id = 0x0922
product_id = 0x0011
interface_class = 0x03           # HID only
max_tape_mm = 12
dpi = 180
print_head_height_mm = 8.2
head_to_cutter_mm = 8.1
synwait = 64

[[devices]]
name = "LabelPoint 350"
vendor_id = 0x0922
product_id = 0x0015
interface_class = 0x03           # HID only
max_tape_mm = 12
dpi = 180
print_head_height_mm = 8.2
head_to_cutter_mm = 8.1
synwait = 64
```

### Full Product ID Table (from labelle source)

| Product ID | Name | Notes |
|------------|------|-------|
| `0x0011` | LabelMANAGER PC | HID interface only |
| `0x001C` | LabelMANAGER PC II | HID interface only |
| `0x0015` | LabelPoint 350 | HID interface only |
| `0x1001` | LabelManager PnP | Pre-modeswitch (mass storage) |
| `0x1002` | LabelManager PnP | Post-modeswitch (printer) |
| `0x1003` | LabelManager 420P | Pre-modeswitch |
| `0x1004` | LabelManager 420P | Post-modeswitch |
| `0x1005` | LabelManager 280 | Pre-modeswitch |
| `0x1006` | LabelManager 280 | Post-modeswitch |
| `0x1007` | LabelManager Wireless PnP | Pre-modeswitch |
| `0x1008` | LabelManager Wireless PnP | Post-modeswitch |
| `0x1009` | MobileLabeler | Unconfirmed |

All share vendor `0x0922` (DYMO).

---

## Label Definitions

Labels are defined as TOML files in a `labels/` directory. Each file describes
one label type with its physical and visual properties. No editor — just drop
a `.toml` file.

### Directory structure
```
labels/
├── 12mm-white.toml
├── 12mm-clear.toml
├── 9mm-white.toml
├── 9mm-yellow.toml
├── 6mm-white.toml
└── 19mm-white.toml
```

### Label definition schema

```toml
# labels/12mm-white.toml

name = "12mm White"
tape_width_mm = 12

# Visual properties (for preview rendering and pixel editor background)
background_color = "#FFFFFF"   # Tape color (white, yellow, clear, etc.)
foreground_color = "#000000"   # Ink color (black, blue, red, white)

# Optional: tape catalog info
tape_type = "D1"               # D1, LetraTAG, etc.
dymo_tape_color_id = 0         # Protocol tape color byte (heat sensitivity)
                               # 0=black-on-white, 1=black-on-blue,
                               # 2=black-on-red, 3=black-on-silver,
                               # 4=black-on-yellow, 5=black-on-gold,
                               # 6=black-on-green, 7=black-on-fluorescent-green,
                               # 8=black-on-fluorescent-red, 9=white-on-clear,
                               # 10=white-on-black, 11=blue-on-white,
                               # 12=red-on-white

# Derived (computed at load time, shown here for documentation):
# height_px = (8 * tape_width_mm / 12) * 8
# For 12mm: height_px = 64
# For 9mm:  height_px = 48
# For 6mm:  height_px = 32
# For 19mm: height_px = ~104 (extrapolated: (8*19/12)*8 ≈ 101.3, rounded)
```

### How labels interact with the system

1. On startup, server scans `labels/` directory
2. Frontend shows label selector dropdown (populated from loaded definitions)
3. Selecting a label sets:
   - Canvas height (from tape width)
   - Preview background/foreground colors
   - Tape color byte sent in ESC C command
4. User can create new label definitions by adding TOML files — no restart needed
   (file watcher or manual reload endpoint)

---

## CUPS Integration Analysis

### How CUPS works with the LabelManager PnP

The Dymo CUPS driver (`printer-driver-dymo` / `dymo-cups-drivers`) provides:
- PPD file: `lmpnp.ppd` (describes capabilities, paper sizes, options)
- Raster filter: `raster2dymolm` (converts CUPS raster → ESC protocol bytes)
- CUPS handles: USB backend communication, job queuing, `usb_modeswitch`

Print pipeline via CUPS:
```
Application → PDF/PS → cups-filters (ghostscript) → CUPS raster → raster2dymolm → ESC bytes → USB backend → printer
```

### Printing a bitmap via CUPS

```bash
# Setup (one-time)
sudo apt install printer-driver-dymo
lpadmin -p label -v 'usb://DYMO/LabelManager%20PnP?serial=SERIAL' \
        -P /usr/share/cups/model/lmpnp.ppd
cupsenable label

# Print a 300x64 landscape bitmap
convert -size 300x64 canvas:white -fill black -draw 'text 10,50 "Hello"' label.png
lp -d label -o landscape label.png
```

### CUPS LabelManager driver options

| Option | Values | Description |
|--------|--------|-------------|
| `PageSize` | `Address_Label`, custom | Label dimensions |
| `DymoCutOptions` | `Cut`, `ChainMarks` | Auto-cut or chain marks between labels |
| `LabelAlignment` | `Left`, `Center`, `Right` | Content alignment |
| `TapeColor` | 0-12 | Heat sensitivity for tape type |
| `DymoTapeWidth` | media type in page header | 6mm, 9mm, 12mm, 19mm, 24mm |
| `landscape` | flag | Rotate 90° (required for bitmaps) |

### Verdict: Should we use CUPS?

| Aspect | Direct USB (nusb) | Via CUPS |
|--------|-------------------|----------|
| **Latency** | Minimal (~ms) | Higher (filter chain, job scheduling) |
| **Control** | Full — exact byte-level protocol | Indirect — CUPS abstracts it |
| **Dependencies** | Zero (pure Rust binary) | cups, printer-driver-dymo, ghostscript |
| **Pixel accuracy** | Exact — we send the bitmap directly | Lossy — CUPS rasterizer may resize/resample |
| **Complexity** | We own the USB protocol (~200 LOC) | CUPS setup, PPD, filter chain |
| **Error handling** | Direct status reads, immediate feedback | Opaque — job fails with generic error |
| **Multi-client** | Must implement locking | Built-in job queue |
| **Network sharing** | Custom (our web UI serves this) | Built-in (IPP/AirPrint) |
| **Cross-platform** | Works anywhere nusb works | Linux/macOS only |

### Recommendation

**Use direct USB (nusb) as primary print path.** Reasons:

1. The whole point of the pixel editor is **exact pixel control**. CUPS adds a
   rasterization step that can resample or shift pixels — defeating the purpose.
2. Zero dependencies on the target (no cups, no printer-driver-dymo, no ghostscript).
3. The protocol is trivial (~6 ESC commands). CUPS adds massive overhead for no benefit.
4. Immediate error feedback (status byte) vs. opaque CUPS job failures.

**Optional CUPS backend** (Phase 3+): If multi-client or AirPrint sharing becomes
desirable, we could add a CUPS backend mode that:
- Installs as a CUPS filter (`raster2labelmanager-rust`)
- Accepts CUPS raster input → our print pipeline
- This would allow `lp` commands and network sharing alongside the web UI
- But this is not needed for the primary use case (dedicated Linux host)

### CUPS as fallback (if nusb fails)

If nusb cannot handle the modeswitch for some reason, the system can fall back to:
1. Let `usb_modeswitch` handle the `0x1001→0x1002` transition (udev rule)
2. Print via `/dev/usb/lp0` (raw device file) — no CUPS needed, just write ESC bytes directly

This is actually simpler than CUPS and still avoids the rasterizer problem.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Browser (any device on LAN)                    │
│  ┌───────────────────────────────────────────┐  │
│  │  HTML5 Canvas Pixel Editor                │  │
│  │  + Widget mode panels (text, dual-line)   │  │
│  │  + Settings panel (tape, speed, margins)  │  │
│  └───────────────────────────────────────────┘  │
│            ▲  WebSocket (live preview)           │
│            │  REST (print, settings CRUD)        │
└────────────┼────────────────────────────────────┘
             │
┌────────────┼────────────────────────────────────┐
│  Rust Server (axum)              Linux host     │
│  ┌─────────┴───────────┐                       │
│  │  Web layer (axum)   │                       │
│  │  - Static files     │                       │
│  │  - REST API         │                       │
│  │  - WebSocket        │                       │
│  └─────────┬───────────┘                       │
│  ┌─────────┴───────────┐                       │
│  │  Label Engine       │                       │
│  │  - Bitmap buffer    │                       │
│  │  - Widget renderers │                       │
│  │  - Preview gen      │                       │
│  └─────────┬───────────┘                       │
│  ┌─────────┴───────────┐                       │
│  │  USB Driver (nusb)  │                       │
│  │  - Mode switch      │                       │
│  │  - ESC protocol     │                       │
│  │  - Print pipeline   │                       │
│  └─────────────────────┘                       │
│  ┌─────────────────────┐                       │
│  │  Config (persisted) │                       │
│  │  - tape width       │                       │
│  │  - margins          │                       │
│  │  - feed speed       │                       │
│  │  - widget presets   │                       │
│  └─────────────────────┘                       │
└─────────────────────────────────────────────────┘
```

---

## Technology Choices

| Layer | Choice | Rationale |
|-------|--------|-----------|
| **Web framework** | `axum` + `tokio` | Mature, lightweight, tower middleware ecosystem. Compiles to small ARM binary. REST-only (no WebSocket — label printing is a short one-shot operation). |
| **Static serving** | `rust-embed` | Single binary deployment. All frontend assets compiled into the binary. Zero filesystem dependency on target. |
| **USB** | `nusb` 0.2.x | Pure Rust, no libusb dependency, async-first, works on Linux ARM. No C toolchain needed on target. |
| **Image handling** | `image` crate | 1-bit bitmap manipulation, PNG encoding for previews. |
| **Frontend** | Vanilla HTML5 Canvas + ES modules + Pico CSS | No Node/npm/bundler. Pixel editor uses raw Canvas API. Pico CSS for classless form styling. All embedded in binary. |
| **Config persistence** | TOML file (`serde` + `toml`) | Human-readable, easy to edit on the host. |
| **Cross-compilation** | `cross` or `cargo-zigbuild` | Target `aarch64-unknown-linux-gnu` from dev machine. |

---

## Features

### Phase 1: Pixel Editor + Print

1. **Pixel Editor Canvas**
   - Grid canvas at exact tape resolution (e.g., 64px tall for 12mm)
   - Variable width (user drags to extend label length)
   - Zoom controls (the pixels are tiny on screen)
   - Draw tools: pencil, eraser, line, rectangle, fill
   - Undo/redo (command stack)
   - Export/import as 1-bit PNG

2. **Settings Panel** (persisted to TOML)
   - Tape width: 6mm / 9mm / 12mm / 19mm (determines canvas height)
   - Horizontal margin (px)
   - Feed speed (if hardware supports it — may be fixed)
   - Print density/tape color setting

3. **Print Pipeline**
   - Canvas → 1-bit bitmap → rotate 270° → chunk into ESC protocol → USB send
   - Status polling with flow control (64-line chunks)
   - USB mode switch (`0x1001` → `0x1002`) if device presents as mass storage

4. **Preview**
   - Client-side: canvas IS the preview (WYSIWYG at hardware resolution)
   - Server-side preview endpoint for widget mode: POST widget config → GET rendered PNG

### Phase 2: Widget Modes

5. **Widget System**
   - Each widget renders to a bitmap at tape height
   - Widgets composited horizontally (like labelle's HorizontallyCombinedRenderEngine)
   - Preview via REST: POST widget config → PNG response

6. **Built-in Widgets**
   - **Single-line text**: Font rendered server-side at exact pixel height, centered
   - **Dual-line text**: Two lines, auto-sized to fit tape height
   - **QR code**: Sized to tape height
   - **Barcode**: CODE128, EAN13, etc.
   - **Image**: Upload, auto-dither to 1-bit

7. **Widget Presets / "Modes"**
   - Pre-configured widget combinations (like physical labeller modes)
   - "Single line" mode: one text widget, full height
   - "Double line" mode: two text widgets, half height each
   - "Address" mode: multi-line small text
   - User can create and save custom presets

### Phase 3: Polish

8. **Font Management**
   - Bundle 2-3 monospace and proportional fonts
   - Server-side text rendering (fontdue or ab_glyph crate)
   - Exact pixel preview — no browser font discrepancies

9. **Label History**
   - Save last N printed labels
   - Re-print from history

10. **Multi-printer Support**
    - Detect multiple connected Dymo devices
    - Per-printer tape size setting

---

## Crate Dependencies (Cargo.toml sketch)

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors"] }
rust-embed = { version = "8", features = ["axum"] }
nusb = "0.2"
image = "0.25"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"

# Phase 2
fontdue = "0.9"           # Font rasterization
qrcode = "0.14"           # QR generation
barcoders = "2"           # Barcode generation
```

---

## Project Structure

```
labelmanagerpnp/
├── Cargo.toml
├── flake.nix                  # Nix dev environment
├── config.toml               # Persisted global settings (devices, margins, etc.)
├── labels/                   # Label definitions (one TOML per label type)
│   ├── 12mm-white.toml
│   ├── 12mm-clear.toml
│   ├── 9mm-white.toml
│   └── 6mm-white.toml
├── src/
│   ├── main.rs               # Entry point, axum setup, static file serving
│   ├── api.rs                # REST endpoints
│   ├── usb/
│   │   ├── mod.rs
│   │   ├── device.rs         # nusb device discovery, mode switch
│   │   └── protocol.rs       # ESC command encoding, print pipeline
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── bitmap.rs         # 1-bit bitmap buffer, rotation, chunking
│   │   ├── widget.rs         # Widget trait + composite renderer
│   │   ├── text.rs           # Text widget (fontdue rasterization)
│   │   ├── qr.rs             # QR widget
│   │   └── barcode.rs        # Barcode widget
│   ├── config.rs             # Global settings struct, TOML persistence
│   └── label.rs              # Label definition loading (scan labels/ dir)
├── static/
│   ├── index.html            # Main page (Pico CSS, ES modules)
│   ├── pico.min.css          # Vendored Pico CSS
│   ├── app.js                # Main app logic
│   ├── canvas-editor.js      # Pixel editor component
│   ├── widgets-panel.js      # Widget mode UI
│   └── settings-panel.js     # Settings UI
└── PLAN.md
```

---

## API Design

### REST

| Method | Path | Description |
|--------|------|-------------|
| `GET /api/status` | | Printer connection status, tape info |
| `GET /api/settings` | | Current settings |
| `PUT /api/settings` | | Update settings (persists to TOML) |
| `POST /api/print` | | Print bitmap (body: PNG or raw 1-bit) |
| `POST /api/preview` | | Render widgets → return PNG preview |
| `GET /api/printers` | | List connected printers |
| `GET /api/labels` | | List available label definitions |
| `GET /api/labels/:name` | | Get specific label definition |
| `GET /api/presets` | | List widget presets |
| `POST /api/presets` | | Save new preset |

---

## USB Driver Implementation Notes

### Mode Switch (mass storage → printer)
```rust
// If device presents as 0x0922:0x1001, send mode switch:
// Endpoint 0x01, message: [0x1b, 0x5a, 0x01]
// Then wait for device to re-enumerate as 0x0922:0x1002
```

### Print Sequence (Rust pseudocode)
```rust
fn print(bitmap: &Bitmap1Bit, tape_mm: u8) -> Result<()> {
    let dev = find_device(0x0922, 0x1002)?;
    let iface = dev.claim_interface(PRINTER_CLASS)?;
    let ep_out = iface.bulk_out_endpoint()?;
    let ep_in = iface.bulk_in_endpoint()?;

    let rotated = bitmap.rotate_270();
    let bytes_per_line = (tape_mm as usize * 8) / 12;

    // Set tape color
    ep_out.write(&[ESC, b'C', 0])?;

    let mut syn_count = 0;
    for column in rotated.columns() {
        let row_bytes = column.pack_msb_first(bytes_per_line);

        // Set bytes per line if changed
        ep_out.write(&[ESC, b'D', row_bytes.len() as u8])?;
        // Send print data
        ep_out.write(&[SYN])?;
        ep_out.write(&row_bytes)?;

        syn_count += 1;
        if syn_count >= 64 {
            // Flow control: request status
            ep_out.write(&[ESC, b'A'])?;
            let _status = ep_in.read(512)?;
            syn_count = 0;
        }
    }

    // Final status
    ep_out.write(&[ESC, b'A'])?;
    let _status = ep_in.read(512)?;
    Ok(())
}
```

---

## Open Questions / Risks

1. **Feed speed control**: The LabelManager PnP may not support variable feed speed.
   The protocol docs don't mention it. Needs hardware testing.

2. **Mode switch from Rust**: nusb should be able to send the mode switch bytes to
   endpoint 0x01 on the mass storage device. If not, we may need a udev rule
   with usb_modeswitch as a fallback (standard Linux setup).

3. **Print head gap**: There's an 8.1mm gap between print head and cutter.
   Labelle compensates with asymmetric margins (0px leading, 2*gap trailing).
   We must replicate this in the print pipeline.

4. **Font quality at 64px**: At these resolutions, anti-aliasing is useless
   (1-bit output). Need to evaluate bitmap fonts vs. fontdue hinting.
   The pixel editor mode sidesteps this entirely.

5. **Canvas width limit**: Labels can theoretically be very long. We should set
   a sensible max (e.g., 1000px = ~141mm ≈ 5.5 inches) to avoid memory issues.

---

## Build & Deploy

```bash
# Development (host machine)
cargo run

# Cross-compile for aarch64 Linux (e.g. Raspberry Pi)
cross build --release --target aarch64-unknown-linux-gnu

# Or for x86_64 Linux
cross build --release --target x86_64-unknown-linux-gnu

# Deploy
scp target/<target>/release/labelmanagerpnp user@host:~/
ssh user@host './labelmanagerpnp'

# Systemd service for auto-start
# [Unit]
# Description=LabelManager PnP Web Interface
# After=network.target
#
# [Service]
# ExecStart=/home/pi/labelmanagerpnp
# WorkingDirectory=/home/pi
# User=pi
# Restart=always
#
# [Install]
# WantedBy=multi-user.target
```

### udev rule (still needed for USB permissions)
```
ACTION=="add", SUBSYSTEMS=="usb", ATTRS{idVendor}=="0922", ATTRS{idProduct}=="1002", MODE="0666"
ACTION=="add", SUBSYSTEMS=="usb", ATTRS{idVendor}=="0922", ATTRS{idProduct}=="1001", RUN+="/usr/sbin/usb_modeswitch -c /etc/usb_modeswitch.d/dymo-labelmanager-pnp.conf"
```

---

## Implementation Order

| # | Task | Depends On | Effort |
|---|------|-----------|--------|
| 1 | Cargo project scaffold + axum hello-world + rust-embed static serving | — | S |
| 2 | USB driver: device discovery + mode switch | — | M |
| 3 | USB driver: ESC protocol + print pipeline | 2 | M |
| 4 | Bitmap module: 1-bit buffer, rotate, chunk | — | S |
| 5 | Config module: TOML read/write, settings struct | — | S |
| 6 | Label module: scan labels/ dir, load definitions | 5 | S |
| 7 | REST API: status, settings, print, labels, preview | 2,3,4,5,6 | M |
| 8 | Frontend: pixel editor canvas (Pico CSS + vanilla JS) | — | L |
| 9 | Integration: pixel editor → print | 7,8 | S |
| 10 | Widget: text renderer (fontdue) | 4 | M |
| 11 | Widget: composite renderer | 10 | S |
| 12 | Widget: QR + barcode | 4 | M |
| 13 | Frontend: widget mode panels | 10,11 | M |
| 14 | Presets system | 5,11 | S |
| 15 | Cross-compilation + deploy | all | S |

**Parallelizable**: Tasks 1-6 can run in parallel. Task 8 (frontend) is independent of backend until integration (9).
