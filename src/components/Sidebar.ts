import { SortMode, TerminalSessionInfo, normalizeTerminalState } from '../types/terminal';
import { SortManager } from '../services/SortManager';

export interface SidebarCallbacks {
  onSelectTerminal: (id: string) => void;
  onPinTopTerminal: (id: string) => void;
  onCloseTerminal: (id: string) => void;
  onSortModeChange: (mode: SortMode) => void;
}

/**
 * Collapsible session tree organizer with 3 sorting modes and double-click to pin top.
 */
export class Sidebar {
  private element: HTMLElement;
  private sessionListContainer: HTMLElement;
  private headerCountSpan: HTMLElement;
  private sortSelect: HTMLSelectElement;
  private sortManager: SortManager;
  private callbacks: SidebarCallbacks;
  private isCollapsed = false;
  private currentSessions: TerminalSessionInfo[] = [];
  private searchQuery = '';
  private activeTerminalId: string | null = null;

  constructor(
    container: HTMLElement,
    callbacks: SidebarCallbacks,
    sortManager: SortManager
  ) {
    this.callbacks = callbacks;
    this.sortManager = sortManager;

    this.element = document.createElement('aside');
    this.element.className = 'sidebar-container';

    const sortSection = document.createElement('div');
    sortSection.className = 'sidebar-section';

    const sortLabel = document.createElement('div');
    sortLabel.className = 'sidebar-section-title';
    sortLabel.textContent = 'SORT / REORDER MODE';

    const sortWrapper = document.createElement('div');
    sortWrapper.className = 'sort-select-wrapper';

    this.sortSelect = document.createElement('select');
    this.sortSelect.className = 'sort-select';

    const sortOptions = [
      { mode: 'running_priority' as SortMode, label: '⚡ Running Priority (Active First)' },
      { mode: 'mru' as SortMode, label: '🕒 Recent Activity (MRU)' },
      { mode: 'creation' as SortMode, label: '📅 Creation Order' }
    ];

    sortOptions.forEach((opt) => {
      const optionEl = document.createElement('option');
      optionEl.value = opt.mode;
      optionEl.textContent = opt.label;
      if (opt.mode === this.sortManager.getSortMode()) {
        optionEl.selected = true;
      }
      this.sortSelect.appendChild(optionEl);
    });

    this.sortSelect.onchange = () => {
      const mode = this.sortSelect.value as SortMode;
      this.sortManager.setSortMode(mode);
      this.callbacks.onSortModeChange(mode);
      this.renderList();
    };

    const sortArrow = document.createElement('span');
    sortArrow.className = 'sort-select-arrow';
    sortArrow.textContent = '▾';

    sortWrapper.appendChild(this.sortSelect);
    sortWrapper.appendChild(sortArrow);

    sortSection.appendChild(sortLabel);
    sortSection.appendChild(sortWrapper);

    const listSection = document.createElement('div');
    listSection.className = 'sidebar-section';
    listSection.style.paddingBottom = '4px';

    this.headerCountSpan = document.createElement('div');
    this.headerCountSpan.className = 'sidebar-section-title';
    this.headerCountSpan.textContent = 'OPEN TERMINALS (0) - [2x click: Pin Top]';
    listSection.appendChild(this.headerCountSpan);

    this.sessionListContainer = document.createElement('div');
    this.sessionListContainer.className = 'session-list-scroll';

    this.element.appendChild(sortSection);
    this.element.appendChild(listSection);
    this.element.appendChild(this.sessionListContainer);

    container.appendChild(this.element);
  }

  /**
   * Toggles sidebar collapsed / expanded state.
   */
  public toggle(): void {
    this.isCollapsed = !this.isCollapsed;
    this.element.classList.toggle('collapsed', this.isCollapsed);
  }

  /**
   * Sets active focused terminal ID.
   */
  public setActiveTerminalId(id: string | null): void {
    this.activeTerminalId = id;
    this.renderList();
  }

  /**
   * Updates sessions list and re-renders tree.
   */
  public updateSessions(sessions: TerminalSessionInfo[]): void {
    this.currentSessions = sessions;
    this.renderList();
  }

  /**
   * Filters displayed sessions with a search query.
   */
  public setSearchQuery(query: string): void {
    this.searchQuery = query.toLowerCase().trim();
    this.renderList();
  }

  private renderList(): void {
    const sorted = this.sortManager.sort(this.currentSessions);
    const filtered = this.searchQuery
      ? sorted.filter(
          (s) =>
            s.title?.toLowerCase().includes(this.searchQuery) ||
            s.id.toLowerCase().includes(this.searchQuery) ||
            s.cwd?.toLowerCase().includes(this.searchQuery) ||
            s.activeCommand?.toLowerCase().includes(this.searchQuery)
        )
      : sorted;

    this.headerCountSpan.textContent = `OPEN TERMINALS (${filtered.length}) - [2x click: Pin Top]`;
    this.sessionListContainer.innerHTML = '';

    filtered.forEach((session) => {
      const card = document.createElement('div');
      card.className = `sidebar-session-card ${
        session.id === this.activeTerminalId ? 'active-terminal' : ''
      }`;

      const state = normalizeTerminalState(session.state);

      if (state === 'STREAMING' || state === 'RUNNING') {
        card.classList.add('state-running');
      } else if (state === 'TERMINATED') {
        card.classList.add('state-terminated');
      }

      card.onclick = () => this.callbacks.onSelectTerminal(session.id);
      card.ondblclick = () => this.callbacks.onPinTopTerminal(session.id);

      const cardHeader = document.createElement('div');
      cardHeader.className = 'session-card-header';

      const titleGroup = document.createElement('div');
      titleGroup.className = 'session-card-title-group';

      const dot = document.createElement('div');
      dot.className = `session-pulse-dot dot-${state.toLowerCase()}`;

      const title = document.createElement('span');
      title.className = 'session-title-text';
      title.textContent = session.title || session.id;

      titleGroup.appendChild(dot);
      titleGroup.appendChild(title);

      const killBtn = document.createElement('button');
      killBtn.className = 'btn-sidebar-kill';
      killBtn.textContent = '✕';
      killBtn.title = 'Close Terminal';
      killBtn.onclick = (e) => {
        e.stopPropagation();
        this.callbacks.onCloseTerminal(session.id);
      };

      cardHeader.appendChild(titleGroup);
      cardHeader.appendChild(killBtn);

      const cardBody = document.createElement('div');
      cardBody.className = 'session-card-body';
      if (session.activeCommand) {
        cardBody.textContent = `$ ${session.activeCommand}`;
      } else if (session.state === 'TERMINATED') {
        const exitCode = session.exitCode !== undefined ? ` (exit: ${session.exitCode})` : '';
        cardBody.textContent = `Terminated${exitCode}`;
      } else if (session.cwd) {
        cardBody.textContent = session.cwd;
      } else {
        cardBody.textContent = 'Idle';
      }

      card.appendChild(cardHeader);
      card.appendChild(cardBody);
      this.sessionListContainer.appendChild(card);
    });
  }
}
