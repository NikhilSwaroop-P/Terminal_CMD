export interface CursorPosition {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Ultra-smooth exponential smear trail renderer for xterm cursor movements.
 */
export class CursorTrail {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private animationFrameId: number | null = null;
  private lastTimestamp = 0;

  private tailX = 0;
  private tailY = 0;
  private headX = 0;
  private headY = 0;
  private cursorWidth = 2;
  private cursorHeight = 16;
  private isInitialized = false;
  private isRunning = false;

  private decayRate = 28.0;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    const context = this.canvas.getContext('2d');
    if (!context) {
      throw new Error('Canvas 2D context not supported');
    }
    this.ctx = context;
  }

  /**
   * Resizes overlay canvas maintaining exact DPI scaling.
   */
  public resize(width: number, height: number): void {
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.max(1, Math.floor(width * dpr));
    this.canvas.height = Math.max(1, Math.floor(height * dpr));
    this.canvas.style.width = `${width}px`;
    this.canvas.style.height = `${height}px`;
    this.ctx.setTransform(1, 0, 0, 1, 0, 0);
    this.ctx.scale(dpr, dpr);
  }

  /**
   * Updates target cursor coordinates.
   */
  public setCursorPosition(pos: CursorPosition): void {
    if (!this.isInitialized) {
      this.headX = pos.x;
      this.headY = pos.y;
      this.tailX = pos.x;
      this.tailY = pos.y;
      this.cursorWidth = Math.max(2, pos.width);
      this.cursorHeight = Math.max(8, pos.height);
      this.isInitialized = true;
      return;
    }

    const dx = pos.x - this.headX;
    const dy = pos.y - this.headY;
    if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) {
      return;
    }

    this.headX = pos.x;
    this.headY = pos.y;
    this.cursorWidth = Math.max(2, pos.width);
    this.cursorHeight = Math.max(8, pos.height);

    if (!this.isRunning) {
      this.isRunning = true;
      this.lastTimestamp = performance.now();
      this.animationFrameId = requestAnimationFrame(this.renderLoop.bind(this));
    }
  }

  /**
   * Updates smooth position decay.
   */
  public updatePhysics(dt: number): boolean {
    const clampedDt = Math.min(dt, 0.05);
    const factor = 1.0 - Math.exp(-this.decayRate * clampedDt);

    this.tailX += (this.headX - this.tailX) * factor;
    this.tailY += (this.headY - this.tailY) * factor;

    const dx = this.headX - this.tailX;
    const dy = this.headY - this.tailY;
    const distSq = dx * dx + dy * dy;

    if (distSq < 0.5) {
      this.tailX = this.headX;
      this.tailY = this.headY;
      return false;
    }
    return true;
  }

  private renderLoop(timestamp: number): void {
    const dt = (timestamp - this.lastTimestamp) / 1000.0;
    this.lastTimestamp = timestamp;

    const isMoving = this.updatePhysics(dt > 0 ? dt : 0.016);
    this.draw();

    if (isMoving) {
      this.animationFrameId = requestAnimationFrame(this.renderLoop.bind(this));
    } else {
      this.isRunning = false;
      this.animationFrameId = null;
      this.clearCanvas();
    }
  }

  private draw(): void {
    const width = parseFloat(this.canvas.style.width) || this.canvas.width;
    const height = parseFloat(this.canvas.style.height) || this.canvas.height;
    this.ctx.clearRect(0, 0, width, height);

    const dx = this.headX - this.tailX;
    const dy = this.headY - this.tailY;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist < 1.0) {
      return;
    }

    const alpha = Math.min(0.75, Math.max(0.15, dist / 12.0));
    const w = Math.max(2, this.cursorWidth);
    const h = this.cursorHeight;

    const gradient = this.ctx.createLinearGradient(
      this.tailX,
      this.tailY,
      this.headX,
      this.headY
    );
    gradient.addColorStop(0, 'rgba(0, 240, 255, 0)');
    gradient.addColorStop(0.5, `rgba(0, 240, 255, ${alpha * 0.45})`);
    gradient.addColorStop(1, `rgba(0, 255, 204, ${alpha})`);

    this.ctx.save();
    this.ctx.shadowBlur = 8;
    this.ctx.shadowColor = 'rgba(0, 240, 255, 0.5)';
    this.ctx.fillStyle = gradient;

    this.ctx.beginPath();
    this.ctx.moveTo(this.tailX, this.tailY);
    this.ctx.lineTo(this.headX, this.headY);
    this.ctx.lineTo(this.headX + w, this.headY);
    this.ctx.lineTo(this.headX + w, this.headY + h);
    this.ctx.lineTo(this.tailX + w, this.tailY + h);
    this.ctx.lineTo(this.tailX, this.tailY + h);
    this.ctx.closePath();
    this.ctx.fill();

    this.ctx.restore();
  }

  private clearCanvas(): void {
    const width = parseFloat(this.canvas.style.width) || this.canvas.width;
    const height = parseFloat(this.canvas.style.height) || this.canvas.height;
    this.ctx.clearRect(0, 0, width, height);
  }

  /**
   * Disposes animation frame and resets state.
   */
  public destroy(): void {
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
    this.isRunning = false;
    this.isInitialized = false;
  }
}
