# labelforge

A Rust-native web interface for the Dymo LabelManager PnP. Inspired by [labelle](https://github.com/labelle-org/labelle), built from scratch to just work.

## Features

- Browser-based label editor with live preview
- Direct USB printing to Dymo LabelManager PnP
- Bundled fonts optimized for small label sizes
- Single static binary, no runtime dependencies

## Quick Start

```sh
cargo build --release
./target/release/labelforge
```

Then open `http://localhost:3000` in your browser.

## Docker

```sh
docker build -t labelforge .
docker run --rm --device /dev/bus/usb -p 3000:3000 labelforge
```

## Configuration

Edit `config.toml` to customize default label settings and font preferences.

## License

MIT — see [LICENSE](LICENSE).

Bundled fonts are distributed under their respective licenses — see [FONTS_LICENSES.md](FONTS_LICENSES.md).
