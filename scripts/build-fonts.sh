#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
FONTS_DIR="$PROJECT_DIR/fonts"
BUILD_DIR="$(mktemp -d)"

trap 'rm -rf "$BUILD_DIR"' EXIT

check_deps() {
    local missing=()
    for cmd in curl tar python3; do
        if ! command -v "$cmd" &>/dev/null; then
            missing+=("$cmd")
        fi
    done
    if ! python3 -c "import fontTools" &>/dev/null; then
        missing+=("python3-fonttools")
    fi
    if [[ ${#missing[@]} -gt 0 ]]; then
        echo "Missing dependencies: ${missing[*]}"
        echo "Install: nix-shell -p python3Packages.fonttools  (or pip install fonttools)"
        exit 1
    fi
}

download_google_font() {
    local base="https://raw.githubusercontent.com/google/fonts/main"
    local path="$1"
    local out="$2"
    echo "  ↓ $out"
    curl -sSfL -o "$FONTS_DIR/$out" "$base/$path"
}

download_github_file() {
    local url="$1"
    local out="$2"
    echo "  ↓ $out"
    curl -sSfL -o "$FONTS_DIR/$out" "$url"
}

build_thermal_sans_mono() {
    echo "Building Thermal Sans Mono (BDF → outlined TTF via fonttools)..."
    local tarball="$BUILD_DIR/thermal.tar.gz"
    curl -sSfL -o "$tarball" \
        "https://github.com/mike42/thermal-sans-mono/releases/download/v0.2/thermal-sans-mono-v0.2.tar.gz"
    tar xzf "$tarball" -C "$BUILD_DIR"

    local bdf="$BUILD_DIR/thermal-sans-mono/thermal-sans-mono-24/thermal-sans-mono-24.bdf"
    local outpath="$FONTS_DIR/ThermalSansMono.ttf"

    python3 "$SCRIPT_DIR/bdf2ttf.py" "$bdf" "$outpath"

    if [[ -f "$outpath" ]]; then
        echo "  ✓ ThermalSansMono.ttf"
    else
        echo "  ✗ Failed to generate ThermalSansMono.ttf"
        return 1
    fi
}

main() {
    check_deps
    mkdir -p "$FONTS_DIR"

    echo "Downloading fonts from Google Fonts..."
    download_google_font "ofl/carlito/Carlito-Regular.ttf" "Carlito-Regular.ttf"
    download_google_font "ofl/carlito/Carlito-Bold.ttf" "Carlito-Bold.ttf"
    download_google_font "ofl/firamono/FiraMono-Regular.ttf" "FiraMono-Regular.ttf"
    download_google_font "ofl/firamono/FiraMono-Bold.ttf" "FiraMono-Bold.ttf"
    download_google_font "ofl/firasanscondensed/FiraSansCondensed-Regular.ttf" "FiraSansCondensed-Regular.ttf"
    download_google_font "ofl/firasanscondensed/FiraSansCondensed-Bold.ttf" "FiraSansCondensed-Bold.ttf"
    download_google_font "ofl/ibmplexmono/IBMPlexMono-Regular.ttf" "IBMPlexMono-Regular.ttf"
    download_google_font "ofl/ibmplexmono/IBMPlexMono-Bold.ttf" "IBMPlexMono-Bold.ttf"
    download_google_font "ofl/inter/Inter%5Bopsz%2Cwght%5D.ttf" "Inter.ttf"
    download_google_font "ofl/notosans/NotoSans%5Bwdth%2Cwght%5D.ttf" "NotoSans.ttf"
    download_google_font "ofl/tiny5/Tiny5-Regular.ttf" "Tiny5-Regular.ttf"

    echo "Downloading IBM Plex Sans..."
    download_github_file "https://raw.githubusercontent.com/IBM/plex/master/packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Regular.ttf" "IBMPlexSans-Regular.ttf"
    download_github_file "https://raw.githubusercontent.com/IBM/plex/master/packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Bold.ttf" "IBMPlexSans-Bold.ttf"

    echo "Downloading DejaVu Sans..."
    local dejavu_tar="$BUILD_DIR/dejavu.tar.bz2"
    curl -sSfL -o "$dejavu_tar" \
        "https://github.com/dejavu-fonts/dejavu-fonts/releases/download/version_2_37/dejavu-fonts-ttf-2.37.tar.bz2"
    tar xjf "$dejavu_tar" -C "$BUILD_DIR"
    cp "$BUILD_DIR/dejavu-fonts-ttf-2.37/ttf/DejaVuSans.ttf" "$FONTS_DIR/"
    cp "$BUILD_DIR/dejavu-fonts-ttf-2.37/ttf/DejaVuSans-Bold.ttf" "$FONTS_DIR/"
    cp "$BUILD_DIR/dejavu-fonts-ttf-2.37/ttf/DejaVuSansMono.ttf" "$FONTS_DIR/"
    cp "$BUILD_DIR/dejavu-fonts-ttf-2.37/ttf/DejaVuSansMono-Bold.ttf" "$FONTS_DIR/"
    echo "  ✓ DejaVu Sans/Mono"

    build_thermal_sans_mono

    echo ""
    echo "Done. $(ls "$FONTS_DIR"/*.ttf | wc -l) font files in $FONTS_DIR/"
}

main "$@"
