import { SortMode, TerminalSessionInfo, normalizeTerminalState } from '../types/terminal';

/**
 * State machine for ordering terminal instances across canvas and sidebar.
 */
export class SortManager {
  private currentMode: SortMode = 'running_priority';
  private pinnedId: string | null = null;
  private activityTimestamps: Map<string, number> = new Map();

  constructor(initialMode: SortMode = 'running_priority') {
    this.currentMode = initialMode;
  }

  /**
   * Sets active sorting mode.
   */
  public setSortMode(mode: SortMode): void {
    this.currentMode = mode;
  }

  /**
   * Gets current sorting mode.
   */
  public getSortMode(): SortMode {
    return this.currentMode;
  }

  /**
   * Pins a specific terminal ID to the top of the canvas.
   */
  public pinToTop(id: string | null): void {
    this.pinnedId = id;
  }

  /**
   * Returns current pinned terminal ID.
   */
  public getPinnedId(): string | null {
    return this.pinnedId;
  }

  /**
   * Records recent command or focus activity for MRU ordering.
   */
  public recordActivity(id: string): void {
    this.activityTimestamps.set(id, Date.now());
  }

  /**
   * Sorts an array of terminal sessions according to active mode and pinned status.
   */
  public sort(terminals: TerminalSessionInfo[]): TerminalSessionInfo[] {
    const list = [...terminals];

    list.sort((a, b) => {
      if (this.pinnedId) {
        if (a.id === this.pinnedId) return -1;
        if (b.id === this.pinnedId) return 1;
      }

      switch (this.currentMode) {
        case 'running_priority': {
          const aState = normalizeTerminalState(a.state);
          const bState = normalizeTerminalState(b.state);
          const aActive = aState === 'RUNNING' || aState === 'STREAMING';
          const bActive = bState === 'RUNNING' || bState === 'STREAMING';
          if (aActive && !bActive) return -1;
          if (!aActive && bActive) return 1;
          return this.compareMru(a.id, b.id);
        }
        case 'mru': {
          return this.compareMru(a.id, b.id);
        }
        case 'creation': {
          const aTime = a.createdAt ? new Date(a.createdAt).getTime() : 0;
          const bTime = b.createdAt ? new Date(b.createdAt).getTime() : 0;
          return bTime - aTime;
        }
        default:
          return 0;
      }
    });

    return list;
  }

  private compareMru(idA: string, idB: string): number {
    const timeA = this.activityTimestamps.get(idA) || 0;
    const timeB = this.activityTimestamps.get(idB) || 0;
    return timeB - timeA;
  }
}
