import { CanvasEditor } from './canvas-editor.js';

const STORAGE_KEY = 'tapir_prefs';
let prefs = {};
let serverDefaults = {};

function loadPrefs() {
    try {
        prefs = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
    } catch {
        prefs = {};
    }
}

function savePrefs() {
    const fontSizeEl = document.getElementById('font-size');
    const saved = {
        label: document.getElementById('label-select').value,
        mode: document.getElementById('mode-select').value,
        canvas_width: document.getElementById('canvas-width').value,
        zoom: document.getElementById('zoom').value,
        auto_feed: document.getElementById('auto-feed').value,
        font: document.getElementById('font-select').value,
        font_size: fontSizeEl ? fontSizeEl.value : null,
        font_weight: document.getElementById('font-weight').value,
        font_italic: document.getElementById('font-italic').getAttribute('aria-pressed') === 'true',
        text_valign: document.getElementById('text-valign').value,
        text_halign: document.getElementById('text-halign').value,
        line_spacing: document.getElementById('line-spacing').value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(saved));
}

const state = {
    labels: [],
    currentLabel: null,
    editor: null,
    productId: null,
};

async function init() {
    loadPrefs();
    await loadLabels();
    await loadSettings();
    await checkStatus();

    const canvas = document.getElementById('editor-canvas');
    const widthInput = document.getElementById('canvas-width');
    const zoomInput = document.getElementById('zoom');

    // Restore saved label (overrides first-in-list default); fall back to server default_label
    const labelName = prefs.label || serverDefaults.default_label;
    if (labelName) {
        const label = state.labels.find(l => l.name === labelName);
        if (label) {
            state.currentLabel = label;
            document.getElementById('label-select').value = label.name;
        }
    }

    // Saved canvas width and zoom override server defaults
    widthInput.value = prefs.canvas_width || serverDefaults.default_canvas_width || widthInput.value;
    zoomInput.value = prefs.zoom || '2';

    const height = pixelHeight(state.currentLabel);
    const margin = state.currentLabel ? (state.currentLabel.margin_px || 0) : 0;
    const width = parseInt(widthInput.value, 10);
    const fg = state.currentLabel ? state.currentLabel.foreground_color : '#000000';
    const bg = state.currentLabel ? state.currentLabel.background_color : '#FFFFFF';

    state.editor = new CanvasEditor(canvas, width, height, parseInt(zoomInput.value, 10), fg, bg, margin);

    widthInput.addEventListener('change', () => {
        state.editor.resize(parseInt(widthInput.value, 10), state.editor.height);
        savePrefs();
    });

    zoomInput.addEventListener('input', () => {
        state.editor.setZoom(parseInt(zoomInput.value, 10));
        savePrefs();
    });

    document.getElementById('label-select').addEventListener('change', (e) => {
        const label = state.labels.find(l => l.name === e.target.value);
        if (label) {
            state.currentLabel = label;
            state.editor.setColors(label.foreground_color, label.background_color);
            state.editor.resize(state.editor.width, pixelHeight(label), label.margin_px || 0);
        }
        savePrefs();
    });

    setupTools();
    setupModes();
    setupActions();

    // Apply saved mode — always dispatch to ensure toolbar visibility matches,
    // even when browser form-restoration already set the select value.
    const modeSelect = document.getElementById('mode-select');
    modeSelect.value = prefs.mode || 'draw';
    modeSelect.dispatchEvent(new Event('change'));

    // Restore saved auto-feed (default: symmetric)
    document.getElementById('auto-feed').value = prefs.auto_feed || 'symmetric';

    document.getElementById('btn-rescan').addEventListener('click', () => checkStatus());
    document.getElementById('btn-reset-prefs').addEventListener('click', () => {
        localStorage.removeItem(STORAGE_KEY);
        location.reload();
    });
}

function pixelHeight(label) {
    if (label && label.height_px) return label.height_px;
    const mm = label ? label.tape_width_mm : 9;
    return mm <= 6 ? 48 : 64;
}

async function loadLabels() {
    try {
        const res = await fetch('/api/labels');
        state.labels = await res.json();
    } catch {
        state.labels = [];
    }

    const select = document.getElementById('label-select');
    select.innerHTML = '';
    for (const label of state.labels) {
        const opt = document.createElement('option');
        opt.value = label.name;
        const px = pixelHeight(label);
        opt.textContent = `${label.name} (${px}px)`;
        select.appendChild(opt);
    }

    state.currentLabel = state.labels[0] || null;
}

async function loadSettings() {
    try {
        const res = await fetch('/api/settings');
        const data = await res.json();
        serverDefaults = data;
        if (data.default_canvas_width) {
            document.getElementById('canvas-width').value = data.default_canvas_width;
        }
    } catch {
    }
}

async function checkStatus() {
    try {
        const res = await fetch('/api/status');
        const data = await res.json();
        const el = document.getElementById('status-indicator');
        if (data.connected) {
            el.textContent = `Connected: ${data.device}`;
            state.productId = data.product_id;
        } else if (data.needs_modeswitch) {
            el.textContent = 'Storage mode — replug device to trigger modeswitch';
            state.productId = null;
        } else {
            el.textContent = 'No printer';
            state.productId = null;
        }
    } catch {
        document.getElementById('status-indicator').textContent = 'Offline';
    }
}

function setupTools() {
    const tools = ['pencil', 'eraser', 'line', 'rect', 'fill'];
    for (const tool of tools) {
        document.getElementById(`tool-${tool}`).addEventListener('click', (e) => {
            for (const t of tools) {
                document.getElementById(`tool-${t}`).removeAttribute('aria-current');
            }
            e.target.setAttribute('aria-current', 'true');
            state.editor.setTool(tool);
        });
    }

    document.getElementById('btn-undo').addEventListener('click', () => state.editor.undo());
    document.getElementById('btn-redo').addEventListener('click', () => state.editor.redo());
    document.getElementById('btn-clear').addEventListener('click', () => state.editor.clear());
}

function setupModes() {
    const modeSelect = document.getElementById('mode-select');
    const toolbarDraw = document.getElementById('toolbar-draw');
    const toolbarText = document.getElementById('toolbar-text');
    const toolbarTextInput = document.getElementById('toolbar-text-input');

    modeSelect.addEventListener('change', () => {
        if (modeSelect.value === 'draw') {
            toolbarDraw.style.display = '';
            toolbarText.style.display = 'none';
            toolbarTextInput.style.display = 'none';
            state.editor.setReadOnly(false);
        } else {
            toolbarText.style.display = '';
            toolbarTextInput.style.display = '';
            toolbarDraw.style.display = 'none';
            state.editor.setReadOnly(true);
            state.editor.clear();
            renderText();
        }
        savePrefs();
    });

    const textInput = document.getElementById('text-input');
    const fontSelect = document.getElementById('font-select');
    const fontSize = document.getElementById('font-size');
    const fontWeight = document.getElementById('font-weight');
    const fontItalic = document.getElementById('font-italic');
    const textValign = document.getElementById('text-valign');
    const textHalign = document.getElementById('text-halign');
    const lineSpacing = document.getElementById('line-spacing');

    let debounceTimer = null;
    const debouncedRender = () => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(renderText, 300);
    };

    textInput.addEventListener('input', debouncedRender);
    fontSelect.addEventListener('change', () => {
        updateWeightOptions();
        updateFontSizeOptions();
        updateItalicAvailability();
        renderText();
        savePrefs();
    });
    fontSize.addEventListener('change', () => { renderText(); savePrefs(); });
    fontWeight.addEventListener('change', () => { renderText(); savePrefs(); });
    fontItalic.addEventListener('click', () => {
        const pressed = fontItalic.getAttribute('aria-pressed') === 'true';
        fontItalic.setAttribute('aria-pressed', pressed ? 'false' : 'true');
        renderText();
        savePrefs();
    });
    textValign.addEventListener('change', () => { renderText(); savePrefs(); });
    textHalign.addEventListener('change', () => { renderText(); savePrefs(); });
    lineSpacing.addEventListener('change', () => { renderText(); savePrefs(); });

    loadFonts();
}

let fontData = { favourites: [], system: [] };

function updateWeightOptions() {
    const fontSelect = document.getElementById('font-select');
    const weightSelect = document.getElementById('font-weight');
    const selectedFamily = fontSelect.value;

    const allFonts = [...fontData.favourites, ...fontData.system];
    const font = allFonts.find(f => f.family === selectedFamily);

    const prevWeight = weightSelect.value;
    weightSelect.innerHTML = '';

    const weightNames = {
        100: 'Thin', 200: 'ExtraLight', 300: 'Light', 400: 'Regular',
        500: 'Medium', 600: 'SemiBold', 700: 'Bold', 800: 'ExtraBold', 900: 'Black',
    };

    const weights = font ? font.weights : [400, 700];
    for (const w of weights) {
        const opt = document.createElement('option');
        opt.value = w;
        opt.textContent = weightNames[w] || `W${w}`;
        weightSelect.appendChild(opt);
    }

    if (weights.includes(parseInt(prevWeight, 10))) {
        weightSelect.value = prevWeight;
    } else if (weights.includes(400)) {
        weightSelect.value = '400';
    }
}

function updateItalicAvailability() {
    const fontSelect = document.getElementById('font-select');
    const fontItalic = document.getElementById('font-italic');
    const selectedFamily = fontSelect.value;

    const allFonts = [...fontData.favourites, ...fontData.system];
    const font = allFonts.find(f => f.family === selectedFamily);

    if (font && font.has_italic) {
        fontItalic.disabled = false;
        fontItalic.style.opacity = '1';
    } else {
        fontItalic.disabled = true;
        fontItalic.style.opacity = '0.3';
        fontItalic.setAttribute('aria-pressed', 'false');
    }
}

function updateFontSizeOptions() {
    const fontSelect = document.getElementById('font-select');
    const fontSizeEl = document.getElementById('font-size');
    const selectedFamily = fontSelect.value;

    const allFonts = [...fontData.favourites, ...fontData.system];
    const font = allFonts.find(f => f.family === selectedFamily);

    const prevSize = parseInt(fontSizeEl.value, 10);

    const sizes = font && font.available_sizes && font.available_sizes.length > 0
        ? font.available_sizes
        : null;

    if (sizes) {
        if (fontSizeEl.tagName === 'INPUT') {
            const sel = document.createElement('select');
            sel.id = 'font-size';
            sel.style.cssText = fontSizeEl.style.cssText;
            for (const s of sizes) {
                const opt = document.createElement('option');
                opt.value = s;
                opt.textContent = `${s}px`;
                sel.appendChild(opt);
            }
            sel.addEventListener('change', () => { renderText(); savePrefs(); });
            fontSizeEl.replaceWith(sel);
            if (sizes.includes(prevSize)) {
                sel.value = prevSize;
            } else {
                sel.value = sizes[sizes.length - 1];
            }
        } else {
            fontSizeEl.innerHTML = '';
            for (const s of sizes) {
                const opt = document.createElement('option');
                opt.value = s;
                opt.textContent = `${s}px`;
                fontSizeEl.appendChild(opt);
            }
            if (sizes.includes(prevSize)) {
                fontSizeEl.value = prevSize;
            } else {
                fontSizeEl.value = sizes[sizes.length - 1];
            }
        }
    } else {
        if (fontSizeEl.tagName === 'SELECT') {
            const input = document.createElement('input');
            input.id = 'font-size';
            input.type = 'number';
            input.min = '6';
            input.max = '128';
            input.value = prevSize || 24;
            input.style.cssText = fontSizeEl.style.cssText;
            input.addEventListener('change', () => { renderText(); savePrefs(); });
            fontSizeEl.replaceWith(input);
        }
    }
}

async function loadFonts() {
    try {
        const res = await fetch('/api/fonts');
        fontData = await res.json();
    } catch {
        fontData = { favourites: [], system: [] };
    }

    const select = document.getElementById('font-select');
    select.innerHTML = '';

    const addGroup = (label, fonts) => {
        if (!fonts.length) return;
        const group = document.createElement('optgroup');
        group.label = label;
        for (const f of fonts) {
            const opt = document.createElement('option');
            opt.value = f.family;
            opt.textContent = f.family;
            group.appendChild(opt);
        }
        select.appendChild(group);
    };

    addGroup('Favourites', fontData.favourites);
    addGroup('System', fontData.system);

    updateWeightOptions();
    updateFontSizeOptions();
    updateItalicAvailability();

    // Restore saved font preferences
    if (prefs.font) {
        const fontSel = document.getElementById('font-select');
        if ([...fontSel.options].some(o => o.value === prefs.font)) {
            fontSel.value = prefs.font;
            updateWeightOptions();
            updateFontSizeOptions();
            updateItalicAvailability();
        }
    }
    if (prefs.font_weight) document.getElementById('font-weight').value = prefs.font_weight;
    if (prefs.font_italic) document.getElementById('font-italic').setAttribute('aria-pressed', 'true');
    if (prefs.font_size) document.getElementById('font-size').value = prefs.font_size;
    if (prefs.text_valign) document.getElementById('text-valign').value = prefs.text_valign;
    if (prefs.text_halign) document.getElementById('text-halign').value = prefs.text_halign;
    if (prefs.line_spacing) document.getElementById('line-spacing').value = prefs.line_spacing;

    if (document.getElementById('mode-select').value === 'text') {
        renderText();
    }
}

let renderAbort = null;

async function renderText() {
    const text = document.getElementById('text-input').value;
    const font = document.getElementById('font-select').value;
    const fontSize = parseInt(document.getElementById('font-size').value, 10);
    const weight = parseInt(document.getElementById('font-weight').value, 10);
    const italic = document.getElementById('font-italic').getAttribute('aria-pressed') === 'true';
    const valign = document.getElementById('text-valign').value;
    const halign = document.getElementById('text-halign').value;
    const lineSpacing = parseInt(document.getElementById('line-spacing').value, 10);
    if (!text || !font) {
        state.editor.clear();
        return;
    }

    if (renderAbort) {
        renderAbort.abort();
    }
    const controller = new AbortController();
    renderAbort = controller;

    try {
        const res = await fetch('/api/render-text', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                text, font, font_size: fontSize, weight, italic,
                height: state.editor.height,
                valign, halign,
                line_spacing: lineSpacing,
            }),
            signal: controller.signal,
        });
        if (!res.ok) {
            const data = await res.json().catch(() => ({}));
            state.editor.showError(data.error || `Render failed (${res.status})`);
            return;
        }
        const blob = await res.blob();
        if (controller.signal.aborted) return;
        await state.editor.loadFromPNG(blob);
        document.getElementById('canvas-width').value = state.editor.width;
    } catch (e) {
        if (e.name !== 'AbortError') throw e;
    } finally {
        if (renderAbort === controller) {
            renderAbort = null;
        }
    }
}

function setupActions() {
    document.getElementById('btn-print').addEventListener('click', async () => {
        if (!state.productId) { alert('No printer connected'); return; }
        const png = await state.editor.toPNG();
        const autoFeed = document.getElementById('auto-feed').value;
        const url = `/api/printers/${state.productId}/print?auto_feed=${autoFeed}`;
        const res = await fetch(url, { method: 'POST', body: png });
        const data = await res.json();
        if (!data.ok) {
            alert(data.error || 'Print failed');
        }
    });

    document.getElementById('btn-feed').addEventListener('click', async () => {
        if (!state.productId) { alert('No printer connected'); return; }
        const res = await fetch(`/api/printers/${state.productId}/feed`, { method: 'POST' });
        const data = await res.json();
        if (!data.ok) {
            alert(data.error || 'Feed failed');
        }
    });

    document.getElementById('btn-export').addEventListener('click', async () => {
        const png = await state.editor.toPNG();
        const url = URL.createObjectURL(new Blob([png], { type: 'image/png' }));
        const a = document.createElement('a');
        a.href = url;
        a.download = 'label.png';
        a.click();
        URL.revokeObjectURL(url);
    });
}

init();
