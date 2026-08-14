export interface CursorPosition {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Spring-damper physics simulation and luminous glow renderer for the Kitty cursor trail.
 */
export class CursorTrail {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private animationFrameId: number | null = null;
  private lastTimestamp = 0;

  private currentX = 0;
  private currentY = 0;
  private targetX = 0;
  private targetY = 0;
  private velocityX = 0;
  private velocityY = 0;
  private cursorWidth = 8;
  private cursorHeight = 16;

  private stiffness = 240.0;
  private damping = 18.0;
  private mass = 1.0;
  private isRunning = false;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    const context = this.canvas.getContext('2d');
    if (!context) {
      throw new Error('Canvas 2D context not supported');
    }
    this.ctx = context;
  }

  /**
   * Resizes the overlay canvas to match terminal container dimensions.
   */
  public resize(width: number, height: number): void {
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.max(1, Math.floor(width * dpr));
    this.canvas.height = Math.max(1, Math.floor(height * dpr));
    this.canvas.style.width = `${width}px`;
    this.canvas.style.height = `${height}px`;
    this.ctx.scale(dpr, dpr);
  }

  /**
   * Updates target cursor coordinates when xterm cursor moves.
   */
  public setCursorPosition(pos: CursorPosition): void {
    this.targetX = pos.x;
    this.targetY = pos.y;
    this.cursorWidth = pos.width;
    this.cursorHeight = pos.height;

    if (!this.isRunning) {
      this.isRunning = true;
      this.lastTimestamp = performance.now();
      this.animationFrameId = requestAnimationFrame(this.renderLoop.bind(this));
    }
  }

  /**
   * Performs one physics step with delta time.
   */
  public updatePhysics(dt: number): boolean {
    const clampedDt = Math.min(dt, 0.05);

    const displacementX = this.currentX - this.targetX;
    const displacementY = this.currentY - this.targetY;

    const forceX = -this.stiffness * displacementX - this.damping * this.velocityX;
    const forceY = -this.stiffness * displacementY - this.damping * this.velocityY;

    const accelX = forceX / this.mass;
    const accelY = forceY / this.mass;

    this.velocityX += accelX * clampedDt;
    this.velocityY += accelY * clampedDt;

    this.currentX += this.velocityX * clampedDt;
    this.currentY += this.velocityY * clampedDt;

    const distSq = displacementX * displacementX + displacementY * displacementY;
    const velSq = this.velocityX * this.velocityX + this.velocityY * this.velocityY;

    if (distSq < 0.05 && velSq < 0.05) {
      this.currentX = this.targetX;
      this.currentY = this.targetY;
      this.velocityX = 0;
      this.velocityY = 0;
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

    const tailCenterX = this.currentX + this.cursorWidth / 2;
    const tailCenterY = this.currentY + this.cursorHeight / 2;
    const headCenterX = this.targetX + this.cursorWidth / 2;
    const headCenterY = this.targetY + this.cursorHeight / 2;

    const dx = headCenterX - tailCenterX;
    const dy = headCenterY - tailCenterY;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist > 1.5) {
      this.ctx.save();
      this.ctx.shadowBlur = 10;
      this.ctx.shadowColor = 'rgba(0, 240, 255, 0.7)';

      const gradient = this.ctx.createLinearGradient(
        tailCenterX,
        tailCenterY,
        headCenterX,
        headCenterY
      );
      gradient.addColorStop(0, 'rgba(0, 240, 255, 0)');
      gradient.addColorStop(0.5, 'rgba(0, 240, 255, 0.45)');
      gradient.addColorStop(1, 'rgba(0, 255, 204, 0.9)');

      this.ctx.beginPath();
      this.ctx.strokeStyle = gradient;
      this.ctx.lineWidth = Math.min(this.cursorHeight * 0.8, 12);
      this.ctx.lineCap = 'round';
      this.ctx.moveTo(tailCenterX, tailCenterY);
      this.ctx.lineTo(headCenterX, headCenterY);
      this.ctx.stroke();

      this.ctx.restore();
    }
  }

  private clearCanvas(): void {
    const width = parseFloat(this.canvas.style.width) || this.canvas.width;
    const height = parseFloat(this.canvas.style.height) || this.canvas.height;
    this.ctx.clearRect(0, 0, width, height);
  }

  /**
   * Cleans up animation frames and listeners.
   */
  public destroy(): void {
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
    this.isRunning = false;
  }
}
