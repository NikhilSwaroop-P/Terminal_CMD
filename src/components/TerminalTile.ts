import { TerminalSessionInfo, TerminalState, ResizeHandleType, normalizeTerminalState } from '../types/terminal';
import { TerminalInstance } from './TerminalInstance';

export interface TerminalTileCallbacks {
  onMinimize: (id: string) => void;
  onMaximize: (id: string) => void;
  onClose: (id: string) => void;
  onRestart: (id: string, cwd: string) => void;
  onFocus: (id: string) => void;
  onResizeStart: (id: string, handle: ResizeHandleType, e: MouseEvent) => void;
  onDimensionChange?: (id: string, cols: number, rows: number) => void;
}

/**
 * Terminal tile card component with Linux top-right controls, dynamic badges, and direct resize handles.
 */
export class TerminalTile {
  public readonly id: string;
  private element: HTMLElement;
  private headerElement: HTMLElement;
  private bodyElement: HTMLElement;
  private dotElement: HTMLElement;
  private titleElement: HTMLElement;
  private cwdElement: HTMLElement;
  private badgeElement: HTMLElement;
  private maxBtnElement: HTMLButtonElement;
  private terminatedBanner: HTMLElement | null = null;

  private terminalInstance: TerminalInstance;
  private callbacks: TerminalTileCallbacks;
  private sessionInfo: TerminalSessionInfo;
  private isMaximized = false;
  private resizeObserver: ResizeObserver | null = null;
  private debounceTimer: number | null = null;

  constructor(
    info: TerminalSessionInfo,
    callbacks: TerminalTileCallbacks,
    customWsUrl?: string,
    token?: string
  ) {
    this.id = info.id;
    this.sessionInfo = { ...info };
    this.callbacks = callbacks;

    this.element = document.createElement('div');
    this.element.className = 'terminal-tile';
    this.element.id = `tile-${this.id}`;

    this.headerElement = document.createElement('div');
    this.headerElement.className = 'tile-header';

    const headerLeft = document.createElement('div');
    headerLeft.className = 'tile-header-left';

    this.dotElement = document.createElement('div');
    this.dotElement.className = 'tile-status-dot dot-idle';

    this.titleElement = document.createElement('span');
    this.titleElement.className = 'tile-title-text';
    this.titleElement.textContent = this.sessionInfo.title || this.id;

    this.cwdElement = document.createElement('span');
    this.cwdElement.className = 'tile-cwd-text';
    this.cwdElement.textContent = this.sessionInfo.cwd ? `(${this.sessionInfo.cwd})` : '';

    headerLeft.appendChild(this.dotElement);
    headerLeft.appendChild(this.titleElement);
    headerLeft.appendChild(this.cwdElement);

    const headerCenter = document.createElement('div');
    headerCenter.className = 'tile-header-center';

    this.badgeElement = document.createElement('span');
    this.badgeElement.className = 'tile-status-badge badge-idle';
    this.badgeElement.textContent = 'IDLE';
    headerCenter.appendChild(this.badgeElement);

    const headerRight = document.createElement('div');
    headerRight.className = 'tile-header-controls';

    const minBtn = document.createElement('button');
    minBtn.className = 'tile-ctrl-btn';
    minBtn.textContent = '_';
    minBtn.title = 'Minimize';
    minBtn.onclick = (e) => {
      e.stopPropagation();
      this.callbacks.onMinimize(this.id);
    };

    this.maxBtnElement = document.createElement('button');
    this.maxBtnElement.className = 'tile-ctrl-btn';
    this.maxBtnElement.textContent = '□';
    this.maxBtnElement.title = 'Maximize / Restore';
    this.maxBtnElement.onclick = (e) => {
      e.stopPropagation();
      this.toggleMaximize();
    };

    const closeBtn = document.createElement('button');
    closeBtn.className = 'tile-ctrl-btn tile-btn-close';
    closeBtn.textContent = '✕';
    closeBtn.title = 'Close Terminal';
    closeBtn.onclick = (e) => {
      e.stopPropagation();
      this.callbacks.onClose(this.id);
    };

    headerRight.appendChild(minBtn);
    headerRight.appendChild(this.maxBtnElement);
    headerRight.appendChild(closeBtn);

    this.headerElement.appendChild(headerLeft);
    this.headerElement.appendChild(headerCenter);
    this.headerElement.appendChild(headerRight);

    this.bodyElement = document.createElement('div');
    this.bodyElement.className = 'tile-body';

    this.element.appendChild(this.headerElement);
    this.element.appendChild(this.bodyElement);

    this.attachResizeHandles();

    this.terminalInstance = new TerminalInstance(
      this.id,
      this.bodyElement,
      customWsUrl,
      token
    );

    this.setupListeners();
    this.updateState(this.sessionInfo);
  }

  private attachResizeHandles(): void {
    const handles: ResizeHandleType[] = [
      'n',
      's',
      'e',
      'w',
      'nw',
      'ne',
      'se',
      'sw'
    ];
    handles.forEach((handle) => {
      const handleEl = document.createElement('div');
      handleEl.className = `resize-handle resize-handle-${handle}`;
      handleEl.onmousedown = (e) => {
        e.preventDefault();
        e.stopPropagation();
        this.callbacks.onResizeStart(this.id, handle, e);
      };
      this.element.appendChild(handleEl);
    });
  }

  private setupListeners(): void {
    this.element.onmousedown = () => {
      this.callbacks.onFocus(this.id);
    };

    this.terminalInstance.onTitleChange((title) => {
      if (title && title.trim()) {
        this.titleElement.textContent = title;
      }
    });

    this.resizeObserver = new ResizeObserver(() => {
      if (this.debounceTimer) {
        window.clearTimeout(this.debounceTimer);
      }
      this.debounceTimer = window.setTimeout(() => {
        const dims = this.terminalInstance.fit();
        this.callbacks.onDimensionChange?.(this.id, dims.cols, dims.rows);
      }, 30);
    });

    this.resizeObserver.observe(this.bodyElement);
  }

  /**
   * Returns root DOM element of the tile.
   */
  public getElement(): HTMLElement {
    return this.element;
  }

  /**
   * Focuses the terminal instance.
   */
  public focus(): void {
    this.element.classList.add('active-focus');
    this.terminalInstance.focus();
  }

  /**
   * Removes focus styling.
   */
  public blur(): void {
    this.element.classList.remove('active-focus');
  }

  /**
   * Updates state metadata and visual badges.
   */
  public updateState(info: Partial<TerminalSessionInfo>): void {
    this.sessionInfo = { ...this.sessionInfo, ...info };

    if (info.title) {
      this.titleElement.textContent = info.title;
    }
    if (info.cwd) {
      this.cwdElement.textContent = `(${info.cwd})`;
    }

    const state: TerminalState = normalizeTerminalState(this.sessionInfo.state);

    this.element.classList.remove(
      'state-streaming',
      'state-running',
      'state-idle',
      'state-terminated'
    );
    this.dotElement.classList.remove(
      'dot-streaming',
      'dot-running',
      'dot-idle',
      'dot-terminated'
    );
    this.badgeElement.classList.remove(
      'badge-streaming',
      'badge-running',
      'badge-idle',
      'badge-terminated'
    );

    if (state === 'STREAMING') {
      this.element.classList.add('state-streaming');
      this.dotElement.classList.add('dot-streaming');
      this.badgeElement.classList.add('badge-streaming');
      this.badgeElement.textContent = 'AGENT STREAMING';
      this.removeTerminatedBanner();
    } else if (state === 'RUNNING') {
      this.element.classList.add('state-running');
      this.dotElement.classList.add('dot-running');
      this.badgeElement.classList.add('badge-running');
      this.badgeElement.textContent = 'RUNNING';
      this.removeTerminatedBanner();
    } else if (state === 'TERMINATED') {
      this.element.classList.add('state-terminated');
      this.dotElement.classList.add('dot-terminated');
      this.badgeElement.classList.add('badge-terminated');
      const exitCode =
        this.sessionInfo.exitCode !== undefined && this.sessionInfo.exitCode !== null
          ? `(exit: ${this.sessionInfo.exitCode})`
          : '';
      this.badgeElement.textContent = `TERMINATED ${exitCode}`.trim();
      this.showTerminatedBanner();
    } else {
      this.element.classList.add('state-idle');
      this.dotElement.classList.add('dot-idle');
      this.badgeElement.classList.add('badge-idle');
      this.badgeElement.textContent = 'IDLE';
      this.removeTerminatedBanner();
    }
  }

  private showTerminatedBanner(): void {
    if (this.terminatedBanner) return;

    this.terminatedBanner = document.createElement('div');
    this.terminatedBanner.className = 'tile-terminated-banner';

    const text = document.createElement('span');
    text.className = 'terminated-text';
    text.textContent = 'Shell session terminated';

    const restartBtn = document.createElement('button');
    restartBtn.className = 'btn-restart-shell';
    restartBtn.textContent = '↺ Restart Shell';
    restartBtn.onclick = (e) => {
      e.stopPropagation();
      this.callbacks.onRestart(this.id, this.sessionInfo.cwd || '');
    };

    this.terminatedBanner.appendChild(text);
    this.terminatedBanner.appendChild(restartBtn);
    this.bodyElement.appendChild(this.terminatedBanner);
  }

  private removeTerminatedBanner(): void {
    if (this.terminatedBanner) {
      this.terminatedBanner.remove();
      this.terminatedBanner = null;
    }
  }

  /**
   * Toggles fullscreen maximize mode.
   */
  public toggleMaximize(): void {
    this.isMaximized = !this.isMaximized;
    this.element.classList.toggle('maximized', this.isMaximized);
    this.maxBtnElement.textContent = this.isMaximized ? '❐' : '□';
    this.callbacks.onMaximize(this.id);
    setTimeout(() => this.fit(), 50);
  }

  /**
   * Refits the inner terminal.
   */
  public fit(): void {
    this.terminalInstance.fit();
  }

  /**
   * Toggles in-tile search overlay.
   */
  public toggleSearch(): void {
    this.terminalInstance.toggleSearch();
  }

  /**
   * Clears terminal screen.
   */
  public clear(): void {
    this.terminalInstance.clear();
  }

  /**
   * Sets explicit height in pixels.
   */
  public setHeight(px: number): void {
    this.element.style.height = `${px}px`;
  }

  /**
   * Disposes and destroys the tile.
   */
  public destroy(): void {
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = null;
    }
    this.terminalInstance.destroy();
    this.element.remove();
  }
}
