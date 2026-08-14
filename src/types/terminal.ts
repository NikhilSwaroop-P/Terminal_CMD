/**
 * Terminal session lifecycle states.
 */
export type TerminalState = 'IDLE' | 'RUNNING' | 'TERMINATED' | 'STREAMING';

/**
 * Normalizes state from backend representation to standard uppercase enum.
 */
export function normalizeTerminalState(state: any, activeCommand?: string | null): TerminalState {
  if (!state) return 'IDLE';
  if (typeof state === 'string') {
    const upper = state.toUpperCase();
    if (upper === 'RUNNING') {
      return activeCommand === null ? 'IDLE' : 'RUNNING';
    }
    if (upper === 'STREAMING') return 'STREAMING';
    if (upper === 'TERMINATED') return 'TERMINATED';
    return 'IDLE';
  }
  if (typeof state === 'object' && state.type) {
    const typeUpper = String(state.type).toUpperCase();
    if (typeUpper === 'STREAMING') return 'STREAMING';
    if (typeUpper === 'TERMINATED') return 'TERMINATED';
    if (typeUpper === 'RUNNING') {
      const cmd = state.data?.command ?? activeCommand;
      if (cmd === null) return 'IDLE';
      return 'RUNNING';
    }
  }
  return 'IDLE';
}

/**
 * Metadata descriptor for a registered PTY session.
 */
export interface TerminalSessionInfo {
  id: string;
  title: string;
  cwd: string;
  pid: number | null;
  state: TerminalState | any;
  activeCommand?: string | null;
  commandStartedAt?: string | null;
  createdAt?: string;
  exitCode?: number | null;
}

/**
 * Options used to spawn a new terminal instance.
 */
export interface TerminalCreateOptions {
  title?: string;
  cwd?: string;
  shell?: string;
  cols?: number;
  rows?: number;
  env?: Record<string, string>;
}

/**
 * Available sorting modes for canvas and sidebar.
 */
export type SortMode = 'running_priority' | 'mru' | 'creation';

/**
 * Grid density columns per row.
 */
export type GridColumns = 1 | 2 | 3 | 4;

/**
 * Direct resize handle positions.
 */
export type ResizeHandleType =
  | 'n'
  | 's'
  | 'e'
  | 'w'
  | 'nw'
  | 'ne'
  | 'se'
  | 'sw';

/**
 * SSE exec event received from backend.
 */
export interface SSEExecEvent {
  event: 'start' | 'stdout' | 'stderr' | 'prompt_waiting' | 'done' | 'error';
  data: string;
}

/**
 * In-tile search options.
 */
export interface SearchOptions {
  caseSensitive?: boolean;
  regex?: boolean;
  wholeWord?: boolean;
}
