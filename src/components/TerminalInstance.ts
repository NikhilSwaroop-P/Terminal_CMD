import { Terminal, ITerminalOptions } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { WebglAddon } from '@xterm/addon-webgl';
import { PtyWebSocket } from '../services/PtyWebSocket';
import { ClipboardService } from '../services/ClipboardService';
import { CursorTrail } from '../effects/CursorTrail';
import { SearchOverlay } from './SearchOverlay';

/**
 * High-performance xterm.js instance with WebGL acceleration, search, and spring cursor trailing.
 */
export class TerminalInstance {
  public readonly id: string;
  private container: HTMLElement;
  private xtermElement: HTMLElement;
  private canvasOverlay: HTMLCanvasElement;
  private term: Terminal;
  private fitAddon: FitAddon;
  private searchAddon: SearchAddon;
  private webglAddon: WebglAddon | null = null;
  private ws: PtyWebSocket;
  private cursorTrail: CursorTrail;
  private searchOverlay: SearchOverlay;
  private isDisposed = false;

  private onCwdChangeListeners: ((cwd: string) => void)[] = [];
  private onTitleChangeListeners: ((title: string) => void)[] = [];

  constructor(id: string, container: HTMLElement, customWsUrl?: string, token?: string) {
    this.id = id;
    this.container = container;

    const wrapper = document.createElement('div');
    wrapper.className = 'xterm-wrapper';
    this.container.appendChild(wrapper);

    this.xtermElement = document.createElement('div');
    this.xtermElement.style.width = '100%';
    this.xtermElement.style.height = '100%';
    wrapper.appendChild(this.xtermElement);

    this.canvasOverlay = document.createElement('canvas');
    this.canvasOverlay.className = 'cursor-trail-canvas';
    wrapper.appendChild(this.canvasOverlay);

    const themeOptions: ITerminalOptions = {
      fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", monospace',
      fontSize: 12.5,
      lineHeight: 1.25,
      cursorBlink: true,
      cursorStyle: 'bar',
      cursorWidth: 2,
      allowProposedApi: true,
      theme: {
        background: '#0d1016',
        foreground: '#f0f6fc',
        cursor: '#00ffcc',
        cursorAccent: '#0a0c10',
        selectionBackground: 'rgba(88, 166, 255, 0.3)',
        black: '#161c24',
        red: '#f85149',
        green: '#3fb950',
        yellow: '#d29922',
        blue: '#58a6ff',
        magenta: '#bc8cff',
        cyan: '#39c5cf',
        white: '#d1d7e0',
        brightBlack: '#6e7681',
        brightRed: '#ff7b72',
        brightGreen: '#56d364',
        brightYellow: '#e3b341',
        brightBlue: '#79c0ff',
        brightMagenta: '#d2a8ff',
        brightCyan: '#56d4dd',
        brightWhite: '#ffffff'
      }
    };

    this.term = new Terminal(themeOptions);
    this.fitAddon = new FitAddon();
    this.searchAddon = new SearchAddon();

    this.term.loadAddon(this.fitAddon);
    this.term.loadAddon(this.searchAddon);

    this.term.open(this.xtermElement);

    try {
      this.webglAddon = new WebglAddon();
      this.term.loadAddon(this.webglAddon);
    } catch {
      this.webglAddon = null;
    }

    this.cursorTrail = new CursorTrail(this.canvasOverlay);
    this.searchOverlay = new SearchOverlay(this.searchAddon);
    this.container.appendChild(this.searchOverlay.getElement());

    this.ws = new PtyWebSocket(this.id, customWsUrl, token);

    this.setupEventPiping();
  }

  private setupEventPiping(): void {
    this.term.onData(data => {
      this.ws.send(data);
    });

    this.ws.onData(data => {
      if (typeof data === 'string') {
        this.term.write(data);
      } else {
        this.term.write(data);
      }
      this.updateCursorTrail();
    });

    this.term.onSelectionChange(() => {
      const selection = this.term.getSelection();
      if (selection) {
        ClipboardService.copyText(selection);
      }
    });

    this.term.onCursorMove(() => {
      this.updateCursorTrail();
    });

    this.term.onTitleChange(title => {
      this.onTitleChangeListeners.forEach(listener => listener(title));
    });

    this.ws.connect();
  }

  /**
   * Recalculates cursor coordinates in pixel space and triggers spring shader.
   */
  public updateCursorTrail(): void {
    if (this.isDisposed) return;

    const buffer = this.term.buffer.active;
    const cursorX = buffer.cursorX;
    const cursorY = buffer.cursorY;

    const cellWidth = (this.term as any)._core?._renderService?.dimensions?.css?.cell?.width || 7.5;
    const cellHeight = (this.term as any)._core?._renderService?.dimensions?.css?.cell?.height || 16;

    const posX = cursorX * cellWidth;
    const posY = cursorY * cellHeight;

    this.cursorTrail.setCursorPosition({
      x: posX,
      y: posY,
      width: cellWidth,
      height: cellHeight
    });
  }

  /**
   * Refits terminal grid to container size.
   */
  public fit(): { cols: number; rows: number } {
    if (this.isDisposed) return { cols: 80, rows: 24 };

    try {
      this.fitAddon.fit();
    } catch {
      return { cols: this.term.cols, rows: this.term.rows };
    }

    const rect = this.container.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      this.cursorTrail.resize(rect.width, rect.height);
    }

    return { cols: this.term.cols, rows: this.term.rows };
  }

  /**
   * Sets focus to xterm buffer.
   */
  public focus(): void {
    if (!this.isDisposed) {
      this.term.focus();
    }
  }

  /**
   * Clears xterm buffer display.
   */
  public clear(): void {
    if (!this.isDisposed) {
      this.term.clear();
    }
  }

  /**
   * Toggles in-tile search widget.
   */
  public toggleSearch(): void {
    this.searchOverlay.toggle();
  }

  /**
   * Returns current column and row metrics.
   */
  public getDimensions(): { cols: number; rows: number } {
    return { cols: this.term.cols, rows: this.term.rows };
  }

  /**
   * Registers title change listener.
   */
  public onTitleChange(listener: (title: string) => void): () => void {
    this.onTitleChangeListeners.push(listener);
    return () => {
      this.onTitleChangeListeners = this.onTitleChangeListeners.filter(l => l !== listener);
    };
  }

  /**
   * Registers working directory change listener.
   */
  public onCwdChange(listener: (cwd: string) => void): () => void {
    this.onCwdChangeListeners.push(listener);
    return () => {
      this.onCwdChangeListeners = this.onCwdChangeListeners.filter(l => l !== listener);
    };
  }

  /**
   * Writes raw string data to the local terminal buffer.
   */
  public write(data: string | Uint8Array): void {
    if (!this.isDisposed) {
      this.term.write(data);
    }
  }

  /**
   * Disposes xterm, WebGL addon, WebSocket, and shaders.
   */
  public destroy(): void {
    this.isDisposed = true;
    this.ws.close();
    this.cursorTrail.destroy();
    if (this.webglAddon) {
      try {
        this.webglAddon.dispose();
      } catch {}
    }
    this.term.dispose();
    this.container.innerHTML = '';
  }
}
