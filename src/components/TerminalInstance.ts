import { Terminal, ITerminalOptions } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { WebglAddon } from '@xterm/addon-webgl';
import { ImageAddon } from '@xterm/addon-image';
import { PtyWebSocket } from '../services/PtyWebSocket';
import { ClipboardService } from '../services/ClipboardService';
import { CursorTrail } from '../effects/CursorTrail';
import { SearchOverlay } from './SearchOverlay';
import { ThemeService } from '../services/ThemeService';

/**
 * High-performance xterm.js instance with WebGL acceleration, theme synchronization, and smooth cursor trailing.
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
  private imageAddon: ImageAddon | null = null;
  private ws: PtyWebSocket;
  private cursorTrail: CursorTrail;
  private searchOverlay: SearchOverlay;
  private themeService: ThemeService = ThemeService.getInstance();
  private unsubscribeTheme: (() => void) | null = null;
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

    const themeService = ThemeService.getInstance();
    const currentTheme = themeService.getTheme();
    const settings = themeService.getSettings();

    const themeOptions: ITerminalOptions = {
      fontFamily:
        '"JetBrainsMono Nerd Font", "JetBrainsMono NF", "FiraCode Nerd Font", "Symbols Nerd Font Mono", "JetBrains Mono", "Fira Code", monospace',
      fontSize: settings.fontSize,
      lineHeight: settings.lineHeight,
      cursorBlink: settings.cursorBlink,
      cursorStyle: settings.cursorStyle,
      cursorWidth: 2,
      allowProposedApi: true,
      theme: currentTheme.terminal
    };

    this.term = new Terminal(themeOptions);
    this.fitAddon = new FitAddon();
    this.searchAddon = new SearchAddon();

    this.term.loadAddon(this.fitAddon);
    this.term.loadAddon(this.searchAddon);

    try {
      this.imageAddon = new ImageAddon();
      this.term.loadAddon(this.imageAddon);
    } catch {
      this.imageAddon = null;
    }

    this.term.open(this.xtermElement);

    try {
      this.webglAddon = new WebglAddon();
      this.term.loadAddon(this.webglAddon);
    } catch {
      this.webglAddon = null;
    }

    this.cursorTrail = new CursorTrail(this.canvasOverlay);
    this.canvasOverlay.style.display = settings.cursorTrail ? 'block' : 'none';

    this.searchOverlay = new SearchOverlay(this.searchAddon);
    this.container.appendChild(this.searchOverlay.getElement());

    this.ws = new PtyWebSocket(this.id, customWsUrl, token);

    this.unsubscribeTheme = themeService.subscribe((theme, newSettings) => {
      if (this.isDisposed) return;
      this.term.options.theme = theme.terminal;
      this.term.options.fontSize = newSettings.fontSize;
      this.term.options.cursorStyle = newSettings.cursorStyle;
      this.term.options.cursorBlink = newSettings.cursorBlink;
      this.canvasOverlay.style.display = newSettings.cursorTrail ? 'block' : 'none';
      this.fit();
    });

    this.term.parser.registerOscHandler(133, (data) => {
      if (data === 'A' || data.startsWith('A;') || data.startsWith('D')) {
        const currentSettings = this.themeService.getSettings();
        this.term.options.cursorStyle = currentSettings.cursorStyle;
      }
      return false;
    });

    this.term.parser.registerCsiHandler({ final: 'q' }, (params) => {
      const p = params[0] || 0;
      if (p === 0) {
        this.term.options.cursorStyle = this.themeService.getSettings().cursorStyle;
        return true;
      }
      return false;
    });

    this.term.parser.registerCsiHandler({ final: 't' }, () => {
      return true;
    });

    this.setupEventPiping();
  }

  private setupEventPiping(): void {
    this.term.onData((data) => {
      if (
        data.startsWith('\x1b]10;') ||
        data.startsWith('\x1b]11;') ||
        data.startsWith('\x1b[4;') ||
        data.startsWith('\x1b[8;') ||
        (data.startsWith('\x1b[') && data.endsWith('t')) ||
        (data.startsWith('\x1b[?') && data.endsWith('c'))
      ) {
        return;
      }
      this.ws.send(data);
    });

    this.ws.onData((data) => {
      this.term.write(data, () => {
        this.updateCursorTrail();
      });
    });

    this.xtermElement.addEventListener('paste', (e: ClipboardEvent) => {
      const text = e.clipboardData?.getData('text');
      if (text) {
        e.preventDefault();
        this.term.paste(text);
      }
    });

    this.term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'v' || e.key === 'V')) {
        if (e.type === 'keydown') {
          ClipboardService.pasteText().then((text) => {
            if (text) {
              this.term.paste(text);
            }
          });
        }
        return false;
      }
      return true;
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

    this.term.onLineFeed(() => {
      this.updateCursorTrail();
    });

    this.term.onTitleChange((title) => {
      this.onTitleChangeListeners.forEach((listener) => listener(title));
    });

    this.ws.connect();
  }

  /**
   * Recalculates cursor coordinates in pixel space and triggers spring shader.
   */
  public updateCursorTrail(): void {
    if (this.isDisposed) return;

    const core = (this.term as any)._core;
    const renderService = core?._renderService;
    const cellWidth =
      renderService?.dimensions?.css?.cell?.width ||
      (this.term.cols > 0 ? this.xtermElement.clientWidth / this.term.cols : 7.8);
    const cellHeight =
      renderService?.dimensions?.css?.cell?.height ||
      (this.term.rows > 0 ? this.xtermElement.clientHeight / this.term.rows : 16);

    const buffer = this.term.buffer.active;
    const cursorX = buffer.cursorX;
    const cursorY = buffer.cursorY;

    const overlayRect = this.canvasOverlay.getBoundingClientRect();
    const screenElement = (this.xtermElement.querySelector('.xterm-screen') ||
      this.xtermElement) as HTMLElement;
    const screenRect = screenElement.getBoundingClientRect();

    const offsetX = Math.max(0, screenRect.left - overlayRect.left);
    const offsetY = Math.max(0, screenRect.top - overlayRect.top);

    const posX = offsetX + cursorX * cellWidth;
    const posY = offsetY + cursorY * cellHeight;

    this.cursorTrail.setCursorPosition({
      x: posX,
      y: posY,
      width: 2,
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
   * Focuses terminal instance.
   */
  public focus(): void {
    if (!this.isDisposed) {
      this.term.focus();
    }
  }

  /**
   * Blurs terminal instance.
   */
  public blur(): void {
    if (!this.isDisposed) {
      this.term.blur();
    }
  }

  /**
   * Clears terminal buffer.
   */
  public clear(): void {
    if (!this.isDisposed) {
      this.term.clear();
    }
  }

  /**
   * Toggles in-tile search widget overlay.
   */
  public toggleSearch(): void {
    this.searchOverlay.toggle();
  }

  /**
   * Registers CWD update listener.
   */
  public onCwdChange(listener: (cwd: string) => void): () => void {
    this.onCwdChangeListeners.push(listener);
    return () => {
      this.onCwdChangeListeners = this.onCwdChangeListeners.filter((l) => l !== listener);
    };
  }

  /**
   * Registers title update listener.
   */
  public onTitleChange(listener: (title: string) => void): () => void {
    this.onTitleChangeListeners.push(listener);
    return () => {
      this.onTitleChangeListeners = this.onTitleChangeListeners.filter((l) => l !== listener);
    };
  }

  /**
   * Disposes xterm instance, addons, and websocket connection.
   */
  public dispose(): void {
    if (this.isDisposed) return;
    this.isDisposed = true;

    if (this.unsubscribeTheme) {
      this.unsubscribeTheme();
      this.unsubscribeTheme = null;
    }

    this.cursorTrail.destroy();
    this.searchOverlay.destroy();
    this.ws.close();

    if (this.imageAddon) {
      try {
        this.imageAddon.dispose();
      } catch {}
    }

    if (this.webglAddon) {
      try {
        this.webglAddon.dispose();
      } catch {}
    }

    try {
      this.fitAddon.dispose();
    } catch {}

    try {
      this.searchAddon.dispose();
    } catch {}

    try {
      this.term.dispose();
    } catch {}
  }

  /**
   * Destroys terminal instance.
   */
  public destroy(): void {
    this.dispose();
  }
}
