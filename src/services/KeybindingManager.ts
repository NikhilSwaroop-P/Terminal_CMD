export interface KeybindingHandlers {
  onNewTerminal?: () => void;
  onSetColumns?: (cols: 1 | 2 | 3 | 4) => void;
  onCloseFocused?: () => void;
  onMinimizeFocused?: () => void;
  onCycleFocusNext?: () => void;
  onCycleFocusPrev?: () => void;
  onToggleSearch?: () => void;
  onToggleSidebar?: () => void;
}

/**
 * Global keyboard shortcuts manager for the desktop canvas.
 */
export class KeybindingManager {
  private handlers: KeybindingHandlers;
  private listener: ((e: KeyboardEvent) => void) | null = null;

  constructor(handlers: KeybindingHandlers) {
    this.handlers = handlers;
  }

  /**
   * Attaches window keydown listener.
   */
  public attach(): void {
    if (this.listener) return;

    this.listener = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.altKey) {
        if (e.key === 'n' || e.key === 'N') {
          e.preventDefault();
          this.handlers.onNewTerminal?.();
        } else if (e.key === 'w' || e.key === 'W') {
          e.preventDefault();
          this.handlers.onCloseFocused?.();
        } else if (e.key === 'm' || e.key === 'M') {
          e.preventDefault();
          this.handlers.onMinimizeFocused?.();
        } else if (e.key === 'b' || e.key === 'B') {
          e.preventDefault();
          this.handlers.onToggleSidebar?.();
        } else if (e.key === '1') {
          e.preventDefault();
          this.handlers.onSetColumns?.(1);
        } else if (e.key === '2') {
          e.preventDefault();
          this.handlers.onSetColumns?.(2);
        } else if (e.key === '3') {
          e.preventDefault();
          this.handlers.onSetColumns?.(3);
        } else if (e.key === '4') {
          e.preventDefault();
          this.handlers.onSetColumns?.(4);
        }
      } else if (e.ctrlKey && e.key === 'Tab') {
        e.preventDefault();
        if (e.shiftKey) {
          this.handlers.onCycleFocusPrev?.();
        } else {
          this.handlers.onCycleFocusNext?.();
        }
      } else if (e.ctrlKey && e.shiftKey && (e.key === 'f' || e.key === 'F')) {
        e.preventDefault();
        this.handlers.onToggleSearch?.();
      }
    };

    window.addEventListener('keydown', this.listener);
  }

  /**
   * Detaches window keydown listener.
   */
  public detach(): void {
    if (this.listener) {
      window.removeEventListener('keydown', this.listener);
      this.listener = null;
    }
  }

  /**
   * Updates registered handlers.
   */
  public updateHandlers(handlers: Partial<KeybindingHandlers>): void {
    this.handlers = { ...this.handlers, ...handlers };
  }
}
