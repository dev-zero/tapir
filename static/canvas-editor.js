export class CanvasEditor {
    constructor(canvas, width, height, zoom) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.width = width;
        this.height = height;
        this.zoom = zoom;
        this.tool = 'pencil';
        this.drawing = false;
        this.startX = 0;
        this.startY = 0;
        this.previewBitmap = null;
        this.history = [];
        this.historyIndex = -1;

        this.bitmap = new Uint8Array(width * height);
        this.applySize();
        this.saveState();
        this.bindEvents();
    }

    applySize() {
        this.canvas.width = this.width * this.zoom;
        this.canvas.height = this.height * this.zoom;
        this.render();
    }

    resize(width, height) {
        const newBitmap = new Uint8Array(width * height);
        const copyW = Math.min(width, this.width);
        const copyH = Math.min(height, this.height);
        for (let y = 0; y < copyH; y++) {
            for (let x = 0; x < copyW; x++) {
                newBitmap[y * width + x] = this.bitmap[y * this.width + x];
            }
        }
        this.width = width;
        this.height = height;
        this.bitmap = newBitmap;
        this.applySize();
        this.saveState();
    }

    setZoom(zoom) {
        this.zoom = zoom;
        this.applySize();
    }

    setTool(tool) {
        this.tool = tool;
    }

    pixelAt(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = Math.floor((e.clientX - rect.left) / this.zoom);
        const y = Math.floor((e.clientY - rect.top) / this.zoom);
        return { x, y };
    }

    setPixel(x, y, value) {
        if (x < 0 || x >= this.width || y < 0 || y >= this.height) return;
        this.bitmap[y * this.width + x] = value ? 1 : 0;
    }

    getPixel(x, y) {
        if (x < 0 || x >= this.width || y < 0 || y >= this.height) return 0;
        return this.bitmap[y * this.width + x];
    }

    render() {
        this.ctx.fillStyle = '#ffffff';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        this.ctx.fillStyle = '#000000';
        for (let y = 0; y < this.height; y++) {
            for (let x = 0; x < this.width; x++) {
                if (this.bitmap[y * this.width + x]) {
                    this.ctx.fillRect(x * this.zoom, y * this.zoom, this.zoom, this.zoom);
                }
            }
        }

        if (this.zoom >= 4) {
            this.ctx.strokeStyle = '#e0e0e0';
            this.ctx.lineWidth = 0.5;
            for (let x = 0; x <= this.width; x++) {
                this.ctx.beginPath();
                this.ctx.moveTo(x * this.zoom, 0);
                this.ctx.lineTo(x * this.zoom, this.canvas.height);
                this.ctx.stroke();
            }
            for (let y = 0; y <= this.height; y++) {
                this.ctx.beginPath();
                this.ctx.moveTo(0, y * this.zoom);
                this.ctx.lineTo(this.canvas.width, y * this.zoom);
                this.ctx.stroke();
            }
        }
    }

    bindEvents() {
        this.canvas.addEventListener('mousedown', (e) => {
            this.drawing = true;
            const { x, y } = this.pixelAt(e);
            this.startX = x;
            this.startY = y;

            if (this.tool === 'line' || this.tool === 'rect') {
                this.previewBitmap = new Uint8Array(this.bitmap);
            } else {
                this.applyTool(x, y);
                this.render();
            }
        });

        this.canvas.addEventListener('mousemove', (e) => {
            if (!this.drawing) return;
            const { x, y } = this.pixelAt(e);

            if (this.tool === 'pencil' || this.tool === 'eraser') {
                this.applyTool(x, y);
                this.render();
            } else if (this.tool === 'line' || this.tool === 'rect') {
                this.bitmap = new Uint8Array(this.previewBitmap);
                if (this.tool === 'line') {
                    this.drawLine(this.startX, this.startY, x, y);
                } else {
                    this.drawRect(this.startX, this.startY, x, y);
                }
                this.render();
            }
        });

        this.canvas.addEventListener('mouseup', (e) => {
            if (!this.drawing) return;
            this.drawing = false;

            if ((this.tool === 'line' || this.tool === 'rect') && this.previewBitmap) {
                const { x, y } = this.pixelAt(e);
                this.bitmap = new Uint8Array(this.previewBitmap);
                if (this.tool === 'line') {
                    this.drawLine(this.startX, this.startY, x, y);
                } else {
                    this.drawRect(this.startX, this.startY, x, y);
                }
                this.previewBitmap = null;
                this.render();
            }

            this.saveState();
        });

        this.canvas.addEventListener('mouseleave', () => {
            if (this.drawing) {
                this.drawing = false;
                if (this.previewBitmap) {
                    this.bitmap = new Uint8Array(this.previewBitmap);
                    this.previewBitmap = null;
                    this.render();
                }
                this.saveState();
            }
        });
    }

    applyTool(x, y) {
        switch (this.tool) {
            case 'pencil':
                this.setPixel(x, y, true);
                break;
            case 'eraser':
                this.setPixel(x, y, false);
                break;
            case 'fill':
                this.floodFill(x, y, this.getPixel(x, y) ? 0 : 1);
                break;
        }
    }

    floodFill(startX, startY, fillValue) {
        const target = this.getPixel(startX, startY);
        if (target === fillValue) return;

        const stack = [{ x: startX, y: startY }];
        while (stack.length > 0) {
            const { x, y } = stack.pop();
            if (x < 0 || x >= this.width || y < 0 || y >= this.height) continue;
            if (this.getPixel(x, y) !== target) continue;

            this.setPixel(x, y, fillValue);
            stack.push({ x: x + 1, y }, { x: x - 1, y }, { x, y: y + 1 }, { x, y: y - 1 });
        }
    }

    drawLine(x0, y0, x1, y1) {
        const dx = Math.abs(x1 - x0);
        const dy = Math.abs(y1 - y0);
        const sx = x0 < x1 ? 1 : -1;
        const sy = y0 < y1 ? 1 : -1;
        let err = dx - dy;

        while (true) {
            this.setPixel(x0, y0, true);
            if (x0 === x1 && y0 === y1) break;
            const e2 = 2 * err;
            if (e2 > -dy) { err -= dy; x0 += sx; }
            if (e2 < dx) { err += dx; y0 += sy; }
        }
    }

    drawRect(x0, y0, x1, y1) {
        const minX = Math.min(x0, x1);
        const maxX = Math.max(x0, x1);
        const minY = Math.min(y0, y1);
        const maxY = Math.max(y0, y1);

        for (let x = minX; x <= maxX; x++) {
            this.setPixel(x, minY, true);
            this.setPixel(x, maxY, true);
        }
        for (let y = minY; y <= maxY; y++) {
            this.setPixel(minX, y, true);
            this.setPixel(maxX, y, true);
        }
    }

    saveState() {
        this.history = this.history.slice(0, this.historyIndex + 1);
        this.history.push(new Uint8Array(this.bitmap));
        this.historyIndex = this.history.length - 1;
    }

    undo() {
        if (this.historyIndex <= 0) return;
        this.historyIndex--;
        this.bitmap = new Uint8Array(this.history[this.historyIndex]);
        this.render();
    }

    redo() {
        if (this.historyIndex >= this.history.length - 1) return;
        this.historyIndex++;
        this.bitmap = new Uint8Array(this.history[this.historyIndex]);
        this.render();
    }

    clear() {
        this.bitmap.fill(0);
        this.render();
        this.saveState();
    }

    toPNG() {
        const offscreen = document.createElement('canvas');
        offscreen.width = this.width;
        offscreen.height = this.height;
        const ctx = offscreen.getContext('2d');
        const imgData = ctx.createImageData(this.width, this.height);

        for (let y = 0; y < this.height; y++) {
            for (let x = 0; x < this.width; x++) {
                const i = (y * this.width + x) * 4;
                const v = this.bitmap[y * this.width + x] ? 0 : 255;
                imgData.data[i] = v;
                imgData.data[i + 1] = v;
                imgData.data[i + 2] = v;
                imgData.data[i + 3] = 255;
            }
        }

        ctx.putImageData(imgData, 0, 0);

        return new Promise((resolve) => {
            offscreen.toBlob(resolve, 'image/png');
        });
    }
}
