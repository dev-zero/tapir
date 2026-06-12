import { CanvasEditor } from './canvas-editor.js';

const state = {
    labels: [],
    currentLabel: null,
    editor: null,
    productId: null,
};

async function init() {
    await loadLabels();
    await loadSettings();
    await checkStatus();

    const canvas = document.getElementById('editor-canvas');
    const widthInput = document.getElementById('canvas-width');
    const zoomInput = document.getElementById('zoom');

    const height = pixelHeight(state.currentLabel);
    const margin = state.currentLabel ? (state.currentLabel.margin_px || 0) : 0;
    const width = parseInt(widthInput.value, 10);
    const fg = state.currentLabel ? state.currentLabel.foreground_color : '#000000';
    const bg = state.currentLabel ? state.currentLabel.background_color : '#FFFFFF';

    state.editor = new CanvasEditor(canvas, width, height, parseInt(zoomInput.value, 10), fg, bg, margin);

    widthInput.addEventListener('change', () => {
        state.editor.resize(parseInt(widthInput.value, 10), state.editor.height);
    });

    zoomInput.addEventListener('input', () => {
        state.editor.setZoom(parseInt(zoomInput.value, 10));
    });

    document.getElementById('label-select').addEventListener('change', (e) => {
        const label = state.labels.find(l => l.name === e.target.value);
        if (label) {
            state.currentLabel = label;
            state.editor.setColors(label.foreground_color, label.background_color);
            state.editor.resize(state.editor.width, pixelHeight(label), label.margin_px || 0);
        }
    });

    setupTools();
    setupModes();
    setupActions();

    document.getElementById('btn-rescan').addEventListener('click', () => checkStatus());
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
        if (data.default_label) {
            const label = state.labels.find(l => l.name === data.default_label);
            if (label) {
                state.currentLabel = label;
                document.getElementById('label-select').value = label.name;
            }
        }
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
    });

    const textInput = document.getElementById('text-input');
    const fontSelect = document.getElementById('font-select');
    const fontSize = document.getElementById('font-size');
    const fontWeight = document.getElementById('font-weight');
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
        renderText();
    });
    fontSize.addEventListener('change', () => renderText());
    fontWeight.addEventListener('change', () => renderText());
    textValign.addEventListener('change', () => renderText());
    textHalign.addEventListener('change', () => renderText());
    lineSpacing.addEventListener('change', () => renderText());

    loadFonts();
}

let fontData = { medium: [], small: [], system: [] };

function updateWeightOptions() {
    const fontSelect = document.getElementById('font-select');
    const weightSelect = document.getElementById('font-weight');
    const selectedFamily = fontSelect.value;

    const allFonts = [...fontData.medium, ...fontData.small, ...fontData.system];
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

function updateFontSizeOptions() {
    const fontSelect = document.getElementById('font-select');
    const fontSizeEl = document.getElementById('font-size');
    const selectedFamily = fontSelect.value;

    const allFonts = [...fontData.medium, ...fontData.small, ...fontData.system];
    const font = allFonts.find(f => f.family === selectedFamily);

    const prevSize = parseInt(fontSizeEl.value, 10);

    if (font && font.native_size) {
        const ns = font.native_size;
        const maxSize = Math.max(64, ns * 4);
        const sizes = [];
        for (let s = ns; s <= maxSize; s += ns) {
            sizes.push(s);
        }

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
            sel.addEventListener('change', () => renderText());
            fontSizeEl.replaceWith(sel);
            if (sizes.includes(prevSize)) {
                sel.value = prevSize;
            } else {
                sel.value = ns;
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
                fontSizeEl.value = ns;
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
            input.addEventListener('change', () => renderText());
            fontSizeEl.replaceWith(input);
        }
    }
}

async function loadFonts() {
    try {
        const res = await fetch('/api/fonts');
        fontData = await res.json();
    } catch {
        fontData = { medium: [], small: [], system: [] };
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

    addGroup('Favourites (medium)', fontData.medium);
    addGroup('Favourites (small)', fontData.small);
    addGroup('System', fontData.system);

    updateWeightOptions();
    updateFontSizeOptions();
}

let renderAbort = null;

async function renderText() {
    const text = document.getElementById('text-input').value;
    const font = document.getElementById('font-select').value;
    const fontSize = parseInt(document.getElementById('font-size').value, 10);
    const weight = parseInt(document.getElementById('font-weight').value, 10);
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
                text, font, font_size: fontSize, weight,
                height: state.editor.height,
                valign, halign,
                line_spacing: lineSpacing,
            }),
            signal: controller.signal,
        });
        if (!res.ok) return;
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
