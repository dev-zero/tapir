export class CanvasEditor {
    constructor(canvas, width, height, zoom, fgColor, bgColor, margin = 0) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.width = width;
        this.height = height;
        this.margin = margin;
        this.zoom = zoom;
        this.fgColor = fgColor || '#000000';
        this.bgColor = bgColor || '#FFFFFF';
        this.tool = 'pencil';
        this.drawing = false;
        this.readOnly = false;
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

    get visualHeight() {
        return this.height + 2 * this.margin;
    }

    applySize() {
        this.canvas.width = this.width * this.zoom;
        this.canvas.height = this.visualHeight * this.zoom;
        this.render();
    }

    resize(width, height, margin) {
        if (margin !== undefined) this.margin = margin;
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

    setColors(fgColor, bgColor) {
        this.fgColor = fgColor;
        this.bgColor = bgColor;
        this.render();
    }

    pixelAt(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = Math.floor((e.clientX - rect.left) / this.zoom);
        const y = Math.floor((e.clientY - rect.top) / this.zoom) - this.margin;
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
        const vh = this.visualHeight;

        if (this.margin > 0) {
            this.ctx.fillStyle = '#d0d0d0';
            this.ctx.fillRect(0, 0, this.canvas.width, this.margin * this.zoom);
            this.ctx.fillRect(0, (this.margin + this.height) * this.zoom, this.canvas.width, this.margin * this.zoom);
        }

        this.ctx.fillStyle = this.bgColor;
        this.ctx.fillRect(0, this.margin * this.zoom, this.canvas.width, this.height * this.zoom);

        this.ctx.fillStyle = this.fgColor;
        for (let y = 0; y < this.height; y++) {
            for (let x = 0; x < this.width; x++) {
                if (this.bitmap[y * this.width + x]) {
                    this.ctx.fillRect(x * this.zoom, (y + this.margin) * this.zoom, this.zoom, this.zoom);
                }
            }
        }

        if (this.zoom >= 4) {
            this.ctx.strokeStyle = '#e0e0e0';
            this.ctx.lineWidth = 0.5;
            for (let x = 0; x <= this.width; x++) {
                this.ctx.beginPath();
                this.ctx.moveTo(x * this.zoom, 0);
                this.ctx.lineTo(x * this.zoom, vh * this.zoom);
                this.ctx.stroke();
            }
            for (let y = 0; y <= vh; y++) {
                this.ctx.beginPath();
                this.ctx.moveTo(0, y * this.zoom);
                this.ctx.lineTo(this.canvas.width, y * this.zoom);
                this.ctx.stroke();
            }
        }
    }

    clampPixel({ x, y }) {
        return {
            x: Math.max(0, Math.min(this.width - 1, x)),
            y: Math.max(0, Math.min(this.height - 1, y)),
        };
    }

    bindEvents() {
        this.canvas.addEventListener('mousedown', (e) => {
            if (this.readOnly) return;
            const p = this.pixelAt(e);
            if (p.x < 0 || p.x >= this.width || p.y < 0 || p.y >= this.height) return;
            this.drawing = true;
            this.startX = p.x;
            this.startY = p.y;

            if (this.tool === 'line' || this.tool === 'rect') {
                this.previewBitmap = new Uint8Array(this.bitmap);
            } else {
                this.applyTool(p.x, p.y);
                this.render();
            }
        });

        this.canvas.addEventListener('mousemove', (e) => {
            if (!this.drawing) return;
            const p = this.clampPixel(this.pixelAt(e));

            if (this.tool === 'pencil' || this.tool === 'eraser') {
                this.applyTool(p.x, p.y);
                this.render();
            } else if (this.tool === 'line' || this.tool === 'rect') {
                this.bitmap = new Uint8Array(this.previewBitmap);
                if (this.tool === 'line') {
                    this.drawLine(this.startX, this.startY, p.x, p.y);
                } else {
                    this.drawRect(this.startX, this.startY, p.x, p.y);
                }
                this.render();
            }
        });

        document.addEventListener('mouseup', (e) => {
            if (!this.drawing) return;
            this.drawing = false;

            if ((this.tool === 'line' || this.tool === 'rect') && this.previewBitmap) {
                const p = this.clampPixel(this.pixelAt(e));
                this.bitmap = new Uint8Array(this.previewBitmap);
                if (this.tool === 'line') {
                    this.drawLine(this.startX, this.startY, p.x, p.y);
                } else {
                    this.drawRect(this.startX, this.startY, p.x, p.y);
                }
                this.previewBitmap = null;
                this.render();
            }

            this.saveState();
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

    setReadOnly(readOnly) {
        this.readOnly = readOnly;
        this.canvas.style.cursor = readOnly ? 'default' : 'crosshair';
    }

    loadFromPNG(blob) {
        const img = new Image();
        const url = URL.createObjectURL(blob);
        return new Promise((resolve) => {
            img.onload = () => {
                this.width = img.width;
                this.height = img.height;
                this.bitmap = new Uint8Array(img.width * img.height);

                const tmp = document.createElement('canvas');
                tmp.width = img.width;
                tmp.height = img.height;
                const tmpCtx = tmp.getContext('2d');
                tmpCtx.drawImage(img, 0, 0);
                const imgData = tmpCtx.getImageData(0, 0, img.width, img.height);

                for (let y = 0; y < img.height; y++) {
                    for (let x = 0; x < img.width; x++) {
                        const i = (y * img.width + x) * 4;
                        const luma = imgData.data[i];
                        this.bitmap[y * this.width + x] = luma < 128 ? 1 : 0;
                    }
                }

                URL.revokeObjectURL(url);
                this.applySize();
                resolve();
            };
            img.src = url;
        });
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
