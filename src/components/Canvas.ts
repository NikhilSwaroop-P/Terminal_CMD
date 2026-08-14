import {
  GridColumns,
  ResizeHandleType,
  TerminalSessionInfo
} from '../types/terminal';
import { TerminalTile, TerminalTileCallbacks } from './TerminalTile';
import { ApiClient, defaultApiClient } from '../services/ApiClient';

export interface CanvasCallbacks {
  onSessionCountChange?: (count: number) => void;
  onActiveTerminalChange?: (id: string | null) => void;
  onRunningCountChange?: (count: number) => void;
  onMinimizeTerminal?: (session: TerminalSessionInfo) => void;
}

interface DragState {
  tileId: string;
  handle: ResizeHandleType;
  startX: number;
  startY: number;
  startWidth: number;
  startHeight: number;
  adjacentRightId: string | null;
  adjacentRightStartWidth: number;
  adjacentLeftId: string | null;
  adjacentLeftStartWidth: number;
  rowTileIds: string[];
}

interface MoveDragState {
  tileId: string;
  startX: number;
  startY: number;
  draggedElement: HTMLElement;
  isMoving: boolean;
}

/**
 * Responsive tiling canvas layout engine managing Hyprland-style window rearranging, grid density, top-insertion, and complementary resizing.
 */
export class Canvas {
  private container: HTMLElement;
  private gridElement: HTMLElement;
  private emptyStateElement: HTMLElement;
  private tiles: Map<string, TerminalTile> = new Map();
  private sessionInfos: Map<string, TerminalSessionInfo> = new Map();
  private activeTerminalId: string | null = null;
  private columns: GridColumns = 2;
  private defaultHeight = 380;
  private apiClient: ApiClient;
  private callbacks: CanvasCallbacks;
  private dragState: DragState | null = null;
  private moveState: MoveDragState | null = null;

  constructor(
    container: HTMLElement,
    callbacks: CanvasCallbacks = {},
    apiClient: ApiClient = defaultApiClient
  ) {
    this.container = container;
    this.callbacks = callbacks;
    this.apiClient = apiClient;

    this.container.className = 'canvas-viewport';

    this.gridElement = document.createElement('div');
    this.gridElement.className = `canvas-grid cols-${this.columns}`;
    this.container.appendChild(this.gridElement);

    this.emptyStateElement = document.createElement('div');
    this.emptyStateElement.className = 'canvas-empty-state';
    this.emptyStateElement.innerHTML = `
      <div class="canvas-empty-icon">⚡</div>
      <div class="canvas-empty-title">No Active Terminal Canvas</div>
      <div class="canvas-empty-hint">Press <kbd>Ctrl+Alt+N</kbd> or click <b>+ NEW TERMINAL</b> to launch a session.</div>
    `;
    this.container.appendChild(this.emptyStateElement);

    this.setupGlobalDragListeners();
    this.updateEmptyState();
  }

  private setupGlobalDragListeners(): void {
    window.addEventListener('mousemove', (e: MouseEvent) => {
      if (this.moveState) {
        const dx = e.clientX - this.moveState.startX;
        const dy = e.clientY - this.moveState.startY;

        if (!this.moveState.isMoving && Math.sqrt(dx * dx + dy * dy) > 6) {
          this.moveState.isMoving = true;
          this.moveState.draggedElement.classList.add('tile-dragging');
          document.body.style.userSelect = 'none';
        }

        if (this.moveState.isMoving) {
          const visibleTiles = Array.from(this.gridElement.children).filter(
            (el) => (el as HTMLElement).style.display !== 'none'
          ) as HTMLElement[];

          for (const el of visibleTiles) {
            if (el === this.moveState.draggedElement) continue;

            const rect = el.getBoundingClientRect();
            if (
              e.clientX >= rect.left &&
              e.clientX <= rect.right &&
              e.clientY >= rect.top &&
              e.clientY <= rect.bottom
            ) {
              const draggedIndex = visibleTiles.indexOf(this.moveState.draggedElement);
              const targetIndex = visibleTiles.indexOf(el);

              if (draggedIndex < targetIndex) {
                this.gridElement.insertBefore(this.moveState.draggedElement, el.nextSibling);
              } else {
                this.gridElement.insertBefore(this.moveState.draggedElement, el);
              }
              break;
            }
          }
        }
        return;
      }

      if (!this.dragState) return;

      const deltaX = e.clientX - this.dragState.startX;
      const deltaY = e.clientY - this.dragState.startY;

      const tile = this.tiles.get(this.dragState.tileId);
      if (!tile) return;

      const tileEl = tile.getElement();

      if (
        this.dragState.handle === 's' ||
        this.dragState.handle === 'se' ||
        this.dragState.handle === 'sw'
      ) {
        const newHeight = Math.max(180, this.dragState.startHeight + deltaY);
        this.dragState.rowTileIds.forEach((rowId) => {
          const rowTile = this.tiles.get(rowId);
          if (rowTile) {
            rowTile.setHeight(newHeight);
            rowTile.fit();
          }
        });
      } else if (
        this.dragState.handle === 'n' ||
        this.dragState.handle === 'ne' ||
        this.dragState.handle === 'nw'
      ) {
        const newHeight = Math.max(180, this.dragState.startHeight - deltaY);
        this.dragState.rowTileIds.forEach((rowId) => {
          const rowTile = this.tiles.get(rowId);
          if (rowTile) {
            rowTile.setHeight(newHeight);
            rowTile.fit();
          }
        });
      }

      if (
        this.dragState.handle === 'e' ||
        this.dragState.handle === 'se' ||
        this.dragState.handle === 'ne'
      ) {
        if (this.dragState.adjacentRightId) {
          const total = this.dragState.startWidth + this.dragState.adjacentRightStartWidth;
          const newW1 = Math.max(260, Math.min(total - 260, this.dragState.startWidth + deltaX));
          const newW2 = total - newW1;
          tileEl.style.width = `${newW1}px`;
          tileEl.style.flex = 'none';

          const adjTile = this.tiles.get(this.dragState.adjacentRightId);
          if (adjTile) {
            const adjEl = adjTile.getElement();
            adjEl.style.width = `${newW2}px`;
            adjEl.style.flex = 'none';
            adjTile.fit();
          }
          tile.fit();
        } else {
          const viewportWidth = this.container.clientWidth - 28;
          const newW = Math.max(260, Math.min(viewportWidth, this.dragState.startWidth + deltaX));
          tileEl.style.width = `${newW}px`;
          tileEl.style.flex = 'none';
          tile.fit();
        }
      } else if (
        this.dragState.handle === 'w' ||
        this.dragState.handle === 'sw' ||
        this.dragState.handle === 'nw'
      ) {
        if (this.dragState.adjacentLeftId) {
          const total = this.dragState.startWidth + this.dragState.adjacentLeftStartWidth;
          const newW1 = Math.max(260, Math.min(total - 260, this.dragState.startWidth - deltaX));
          const newWLeft = total - newW1;
          tileEl.style.width = `${newW1}px`;
          tileEl.style.flex = 'none';

          const leftTile = this.tiles.get(this.dragState.adjacentLeftId);
          if (leftTile) {
            const leftEl = leftTile.getElement();
            leftEl.style.width = `${newWLeft}px`;
            leftEl.style.flex = 'none';
            leftTile.fit();
          }
          tile.fit();
        } else {
          const viewportWidth = this.container.clientWidth - 28;
          const newW = Math.max(260, Math.min(viewportWidth, this.dragState.startWidth - deltaX));
          tileEl.style.width = `${newW}px`;
          tileEl.style.flex = 'none';
          tile.fit();
        }
      }
    });

    window.addEventListener('mouseup', () => {
      if (this.moveState) {
        if (this.moveState.isMoving) {
          this.moveState.draggedElement.classList.remove('tile-dragging');
          document.body.style.userSelect = '';
          this.refitAll();
        }
        this.moveState = null;
      }

      if (this.dragState) {
        this.tiles.forEach((t) => t.getElement().classList.remove('is-resizing'));
        this.dragState.rowTileIds.forEach((id) => {
          this.tiles.get(id)?.fit();
        });
        this.dragState = null;
      }
    });
  }

  private getRowContext(tileId: string): {
    rowTileIds: string[];
    adjacentRightId: string | null;
    adjacentLeftId: string | null;
  } {
    const visibleTiles = Array.from(this.gridElement.children).filter(
      (el) => (el as HTMLElement).style.display !== 'none'
    ) as HTMLElement[];

    const index = visibleTiles.findIndex((el) => el.id === `tile-${tileId}`);
    if (index === -1) {
      return { rowTileIds: [tileId], adjacentRightId: null, adjacentLeftId: null };
    }

    const cols = this.columns;
    const rowIndex = Math.floor(index / cols);
    const rowStart = rowIndex * cols;
    const rowEnd = Math.min(visibleTiles.length, (rowIndex + 1) * cols);

    const rowTiles = visibleTiles.slice(rowStart, rowEnd);
    const rowTileIds = rowTiles.map((el) => el.id.replace('tile-', ''));

    const adjacentRightId =
      index + 1 < rowEnd ? visibleTiles[index + 1].id.replace('tile-', '') : null;
    const adjacentLeftId =
      index - 1 >= rowStart ? visibleTiles[index - 1].id.replace('tile-', '') : null;

    return { rowTileIds, adjacentRightId, adjacentLeftId };
  }

  /**
   * Spawns or adds a terminal tile at the top of the canvas.
   */
  public addTerminal(
    info: TerminalSessionInfo,
    customWsUrl?: string,
    token?: string
  ): TerminalTile {
    if (this.tiles.has(info.id)) {
      const existing = this.tiles.get(info.id)!;
      existing.updateState(info);
      return existing;
    }

    const callbacks: TerminalTileCallbacks = {
      onMinimize: (id) => this.minimizeTerminal(id),
      onMaximize: () => this.refitAll(),
      onClose: (id) => this.closeTerminal(id),
      onRestart: (id, cwd) => this.restartTerminal(id, cwd),
      onFocus: (id) => this.focusTerminal(id),
      onResizeStart: (id, handle, e) => this.startDrag(id, handle, e),
      onMoveStart: (id, e) => this.startMove(id, e),
      onDimensionChange: (id, cols, rows) => {
        this.apiClient.resizeTerminal(id, cols, rows).catch(() => {});
      }
    };

    const tile = new TerminalTile(info, callbacks, customWsUrl, token);
    tile.setHeight(this.defaultHeight);

    this.tiles.set(info.id, tile);
    this.sessionInfos.set(info.id, info);

    if (this.gridElement.firstChild) {
      this.gridElement.insertBefore(tile.getElement(), this.gridElement.firstChild);
    } else {
      this.gridElement.appendChild(tile.getElement());
    }

    this.updateEmptyState();
    this.focusTerminal(info.id);
    this.notifyCounts();

    setTimeout(() => {
      tile.fit();
    }, 50);

    return tile;
  }

  private startMove(id: string, e: MouseEvent): void {
    const tile = this.tiles.get(id);
    if (!tile) return;

    this.moveState = {
      tileId: id,
      startX: e.clientX,
      startY: e.clientY,
      draggedElement: tile.getElement(),
      isMoving: false
    };
  }

  private startDrag(id: string, handle: ResizeHandleType, e: MouseEvent): void {
    const tile = this.tiles.get(id);
    if (!tile) return;

    this.tiles.forEach((t) => t.getElement().classList.add('is-resizing'));

    const rect = tile.getElement().getBoundingClientRect();
    const { rowTileIds, adjacentRightId, adjacentLeftId } = this.getRowContext(id);

    let adjacentRightStartWidth = 0;
    if (adjacentRightId) {
      const rightTile = this.tiles.get(adjacentRightId);
      if (rightTile) {
        adjacentRightStartWidth = rightTile.getElement().getBoundingClientRect().width;
      }
    }

    let adjacentLeftStartWidth = 0;
    if (adjacentLeftId) {
      const leftTile = this.tiles.get(adjacentLeftId);
      if (leftTile) {
        adjacentLeftStartWidth = leftTile.getElement().getBoundingClientRect().width;
      }
    }

    this.dragState = {
      tileId: id,
      handle,
      startX: e.clientX,
      startY: e.clientY,
      startWidth: rect.width,
      startHeight: rect.height,
      adjacentRightId,
      adjacentRightStartWidth,
      adjacentLeftId,
      adjacentLeftStartWidth,
      rowTileIds
    };
  }

  /**
   * Sets grid density columns (1, 2, 3, or 4).
   */
  public setColumns(cols: GridColumns): void {
    this.columns = cols;
    this.gridElement.className = `canvas-grid cols-${this.columns}`;
    this.tiles.forEach((tile) => {
      const el = tile.getElement();
      el.style.width = '';
      el.style.flex = '';
    });
    this.refitAll();
  }

  /**
   * Sets default height in pixels for all non-customized tiles.
   */
  public setDefaultHeight(px: number): void {
    this.defaultHeight = px;
    this.tiles.forEach((tile) => {
      tile.setHeight(px);
    });
    this.refitAll();
  }

  /**
   * Focuses a specific terminal tile.
   */
  public focusTerminal(id: string): void {
    if (this.activeTerminalId && this.activeTerminalId !== id) {
      const prev = this.tiles.get(this.activeTerminalId);
      if (prev) prev.blur();
    }
    this.activeTerminalId = id;
    const current = this.tiles.get(id);
    if (current) {
      current.focus();
      current.getElement().scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
    this.callbacks.onActiveTerminalChange?.(id);
  }

  /**
   * Minimizes a terminal tile with smooth Hyprland slide animation.
   */
  public minimizeTerminal(id: string): void {
    const info = this.sessionInfos.get(id);
    const tile = this.tiles.get(id);
    if (!info || !tile) return;

    const el = tile.getElement();
    el.classList.add('anim-minimizing');
    setTimeout(() => {
      el.style.display = 'none';
      el.classList.remove('anim-minimizing');
      this.callbacks.onMinimizeTerminal?.(info);

      if (this.activeTerminalId === id) {
        this.cycleFocus(true);
      }
    }, 240);
  }

  /**
   * Restores a minimized terminal tile back into the canvas with pop-in spring animation.
   */
  public restoreTerminal(id: string): void {
    const tile = this.tiles.get(id);
    if (tile) {
      const el = tile.getElement();
      el.style.display = 'flex';
      el.classList.add('anim-restoring');
      setTimeout(() => {
        el.classList.remove('anim-restoring');
      }, 280);
      this.focusTerminal(id);
      tile.fit();
    }
  }

  /**
   * Closes and kills a terminal tile.
   */
  public async closeTerminal(id: string): Promise<void> {
    const tile = this.tiles.get(id);
    if (tile) {
      tile.destroy();
      this.tiles.delete(id);
      this.sessionInfos.delete(id);
    }

    try {
      await this.apiClient.deleteTerminal(id);
    } catch {}

    this.updateEmptyState();
    this.notifyCounts();

    if (this.activeTerminalId === id) {
      this.cycleFocus(true);
    }
  }

  /**
   * Restarts a terminated shell session in the same CWD.
   */
  public async restartTerminal(id: string, cwd: string): Promise<void> {
    await this.closeTerminal(id);
    try {
      const newSession = await this.apiClient.createTerminal({ cwd });
      this.addTerminal(newSession);
    } catch {}
  }

  /**
   * Pins a terminal to the top position and smoothly scrolls it into view.
   */
  public pinAndScrollToTop(id: string): void {
    const tile = this.tiles.get(id);
    if (!tile) return;

    const el = tile.getElement();
    if (this.gridElement.firstChild !== el) {
      this.gridElement.insertBefore(el, this.gridElement.firstChild);
    }
    this.focusTerminal(id);
    el.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  /**
   * Cycles focus to next or previous terminal tile.
   */
  public cycleFocus(forward = true): void {
    const ids = Array.from(this.tiles.keys());
    if (ids.length === 0) return;

    if (!this.activeTerminalId) {
      this.focusTerminal(ids[0]);
      return;
    }

    const currentIndex = ids.indexOf(this.activeTerminalId);
    let nextIndex = forward ? currentIndex + 1 : currentIndex - 1;
    if (nextIndex >= ids.length) nextIndex = 0;
    if (nextIndex < 0) nextIndex = ids.length - 1;

    this.focusTerminal(ids[nextIndex]);
  }

  /**
   * Returns active focused terminal ID.
   */
  public getActiveTerminalId(): string | null {
    return this.activeTerminalId;
  }

  /**
   * Returns a map of all terminal sessions.
   */
  public getSessionInfos(): TerminalSessionInfo[] {
    return Array.from(this.sessionInfos.values());
  }

  /**
   * Synchronizes external list of terminals.
   */
  public syncTerminals(terminals: TerminalSessionInfo[]): void {
    const remoteIds = new Set(terminals.map((t) => t.id));

    terminals.forEach((info) => {
      if (!this.tiles.has(info.id)) {
        this.addTerminal(info);
      } else {
        this.sessionInfos.set(info.id, info);
        const tile = this.tiles.get(info.id);
        if (tile) {
          tile.updateState(info);
        }
      }
    });

    for (const [id, tile] of this.tiles.entries()) {
      if (!remoteIds.has(id)) {
        tile.destroy();
        this.tiles.delete(id);
        this.sessionInfos.delete(id);
        if (this.activeTerminalId === id) {
          this.activeTerminalId = null;
        }
      }
    }

    this.updateEmptyState();
    this.notifyCounts();
  }

  /**
   * Toggles in-tile search on focused terminal.
   */
  public toggleSearchOnFocused(): void {
    if (this.activeTerminalId) {
      const tile = this.tiles.get(this.activeTerminalId);
      tile?.toggleSearch();
    }
  }

  /**
   * Refits all visible terminal instances.
   */
  public refitAll(): void {
    setTimeout(() => {
      this.tiles.forEach((tile) => tile.fit());
    }, 50);
  }

  private updateEmptyState(): void {
    if (this.tiles.size === 0) {
      this.emptyStateElement.style.display = 'flex';
      this.gridElement.style.display = 'none';
    } else {
      this.emptyStateElement.style.display = 'none';
      this.gridElement.style.display = 'flex';
    }
  }

  private notifyCounts(): void {
    this.callbacks.onSessionCountChange?.(this.sessionInfos.size);
    let runningCount = 0;
    this.sessionInfos.forEach((info) => {
      const state = String(info.state).toUpperCase();
      if (state === 'RUNNING' || state === 'STREAMING') {
        runningCount++;
      }
    });
    this.callbacks.onRunningCountChange?.(runningCount);
  }
}
