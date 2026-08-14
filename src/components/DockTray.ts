import { TerminalSessionInfo, normalizeTerminalState } from '../types/terminal';

export interface DockTrayCallbacks {
  onRestoreTerminal: (id: string) => void;
}

/**
 * Minimized terminal window dock tray rendered at the bottom of the viewport.
 */
export class DockTray {
  private element: HTMLElement;
  private chipsGroup: HTMLElement;
  private rightInfoSpan: HTMLElement;
  private minimizedSessions: Map<string, TerminalSessionInfo> = new Map();
  private callbacks: DockTrayCallbacks;

  constructor(container: HTMLElement, callbacks: DockTrayCallbacks) {
    this.callbacks = callbacks;

    this.element = document.createElement('footer');
    this.element.className = 'dock-container';

    const left = document.createElement('div');
    left.className = 'dock-left';

    const label = document.createElement('span');
    label.className = 'dock-label';
    label.textContent = 'MINIMIZED DOCK:';

    this.chipsGroup = document.createElement('div');
    this.chipsGroup.className = 'dock-chips-group';

    left.appendChild(label);
    left.appendChild(this.chipsGroup);

    const right = document.createElement('div');
    right.className = 'dock-right';

    this.rightInfoSpan = document.createElement('span');
    this.rightInfoSpan.textContent = 'Tiling Canvas (2 cols) | Scrollable';
    right.appendChild(this.rightInfoSpan);

    this.element.appendChild(left);
    this.element.appendChild(right);

    container.appendChild(this.element);
  }

  /**
   * Adds a terminal session to minimized dock tray.
   */
  public addMinimized(session: TerminalSessionInfo): void {
    this.minimizedSessions.set(session.id, session);
    this.renderChips();
  }

  /**
   * Removes a terminal session from minimized dock tray.
   */
  public removeMinimized(id: string): void {
    this.minimizedSessions.delete(id);
    this.renderChips();
  }

  /**
   * Updates canvas status text on the right side of the dock.
   */
  public setCanvasInfo(info: string): void {
    this.rightInfoSpan.textContent = info;
  }

  private renderChips(): void {
    this.chipsGroup.innerHTML = '';
    this.minimizedSessions.forEach((session) => {
      const chip = document.createElement('div');
      chip.className = 'dock-chip';
      chip.title = 'Click to restore window';
      chip.onclick = () => {
        this.removeMinimized(session.id);
        this.callbacks.onRestoreTerminal(session.id);
      };

      const state = normalizeTerminalState(session.state);
      const dot = document.createElement('div');
      dot.className = `session-pulse-dot dot-${state.toLowerCase()}`;

      const text = document.createElement('span');
      text.className = 'dock-chip-text';
      text.textContent = `${session.title || session.id} (${state.toLowerCase()})`;

      chip.appendChild(dot);
      chip.appendChild(text);
      this.chipsGroup.appendChild(chip);
    });
  }
}
