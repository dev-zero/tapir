import { CanvasEditor } from './canvas-editor.js';

const state = {
    labels: [],
    currentLabel: null,
    editor: null,
};

async function init() {
    await loadLabels();
    await checkStatus();

    const canvas = document.getElementById('editor-canvas');
    const widthInput = document.getElementById('canvas-width');
    const zoomInput = document.getElementById('zoom');

    const height = state.currentLabel ? pixelHeight(state.currentLabel.tape_width_mm) : 64;
    const width = parseInt(widthInput.value, 10);

    state.editor = new CanvasEditor(canvas, width, height, parseInt(zoomInput.value, 10));

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
            state.editor.resize(state.editor.width, pixelHeight(label.tape_width_mm));
        }
    });

    setupTools();
    setupActions();
}

function pixelHeight(tapeMm) {
    return ((8 * tapeMm) / 12) * 8;
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
        opt.textContent = `${label.name} (${label.tape_width_mm}mm)`;
        select.appendChild(opt);
    }

    state.currentLabel = state.labels[0] || null;
}

async function checkStatus() {
    try {
        const res = await fetch('/api/status');
        const data = await res.json();
        const el = document.getElementById('status-indicator');
        el.textContent = data.connected ? `Connected: ${data.device}` : 'Disconnected';
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

function setupActions() {
    document.getElementById('btn-print').addEventListener('click', async () => {
        const png = state.editor.toPNG();
        const res = await fetch('/api/print', { method: 'POST', body: png });
        const data = await res.json();
        if (!data.ok) {
            alert(data.error || 'Print failed');
        }
    });

    document.getElementById('btn-export').addEventListener('click', () => {
        const png = state.editor.toPNG();
        const url = URL.createObjectURL(new Blob([png], { type: 'image/png' }));
        const a = document.createElement('a');
        a.href = url;
        a.download = 'label.png';
        a.click();
        URL.revokeObjectURL(url);
    });
}

init();
