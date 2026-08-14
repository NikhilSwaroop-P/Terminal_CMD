import { GridColumns } from '../types/terminal';

export interface TopBarCallbacks {
  onToggleSidebar: () => void;
  onNewTerminal: () => void;
  onSetColumns: (cols: GridColumns) => void;
  onSetDefaultHeight: (heightPx: number) => void;
  onSearchQueryChange: (query: string) => void;
  onOpenSettings?: () => void;
}

/**
 * Top navigation bar with density controls, height selector, search, and running status badge.
 */
export class TopBar {
  private element: HTMLElement;
  private runningBadgeText: HTMLSpanElement;
  private segmentButtons: Map<GridColumns, HTMLButtonElement> = new Map();
  private callbacks: TopBarCallbacks;
  private currentCols: GridColumns = 2;

  constructor(container: HTMLElement, callbacks: TopBarCallbacks) {
    this.callbacks = callbacks;
    this.element = document.createElement('header');
    this.element.className = 'topbar-container';

    const left = document.createElement('div');
    left.className = 'topbar-left';

    const sidebarToggleBtn = document.createElement('button');
    sidebarToggleBtn.className = 'btn-icon';
    sidebarToggleBtn.innerHTML = '☰';
    sidebarToggleBtn.title = 'Toggle Session Sidebar (Ctrl+Alt+B)';
    sidebarToggleBtn.onclick = () => this.callbacks.onToggleSidebar();

    const newTermBtn = document.createElement('button');
    newTermBtn.className = 'btn-primary';
    newTermBtn.innerHTML = '<span>+</span> NEW TERMINAL';
    newTermBtn.title = 'Spawn New Terminal (Ctrl+Alt+N)';
    newTermBtn.onclick = () => {
      newTermBtn.blur();
      this.callbacks.onNewTerminal();
    };

    left.appendChild(sidebarToggleBtn);
    left.appendChild(newTermBtn);

    const center = document.createElement('div');
    center.className = 'topbar-center';

    const colsSegment = document.createElement('div');
    colsSegment.className = 'topbar-segment';

    const colsLabel = document.createElement('span');
    colsLabel.className = 'topbar-segment-label';
    colsLabel.textContent = 'Cols/Row:';
    colsSegment.appendChild(colsLabel);

    const colValues: GridColumns[] = [1, 2, 3, 4];
    colValues.forEach((val) => {
      const btn = document.createElement('button');
      btn.className = `segment-btn ${val === this.currentCols ? 'active' : ''}`;
      btn.textContent = val.toString();
      btn.title = `Switch to ${val} column${val > 1 ? 's' : ''} per row (Ctrl+Alt+${val})`;
      btn.onclick = () => {
        this.setColumns(val);
        this.callbacks.onSetColumns(val);
      };
      this.segmentButtons.set(val, btn);
      colsSegment.appendChild(btn);
    });

    const heightSegment = document.createElement('div');
    heightSegment.className = 'topbar-segment';

    const heightLabel = document.createElement('span');
    heightLabel.className = 'topbar-segment-label';
    heightLabel.textContent = 'Default Height:';
    heightSegment.appendChild(heightLabel);

    const heightSelect = document.createElement('select');
    heightSelect.className = 'dropdown-select';
    const heights = [
      { label: '280px', val: 280 },
      { label: '340px', val: 340 },
      { label: '380px', val: 380 },
      { label: '450px', val: 450 },
      { label: '600px', val: 600 }
    ];
    heights.forEach((h) => {
      const opt = document.createElement('option');
      opt.value = h.val.toString();
      opt.textContent = h.label;
      if (h.val === 380) opt.selected = true;
      heightSelect.appendChild(opt);
    });
    heightSelect.onchange = () => {
      this.callbacks.onSetDefaultHeight(parseInt(heightSelect.value, 10));
    };
    heightSegment.appendChild(heightSelect);

    const searchWrapper = document.createElement('div');
    searchWrapper.className = 'search-input-wrapper';

    const searchIcon = document.createElement('span');
    searchIcon.className = 'search-icon-placeholder';
    searchIcon.textContent = '🔍';

    const searchInput = document.createElement('input');
    searchInput.type = 'text';
    searchInput.className = 'search-input';
    searchInput.placeholder = 'Search sessions...';
    searchInput.oninput = () => {
      this.callbacks.onSearchQueryChange(searchInput.value);
    };

    searchWrapper.appendChild(searchIcon);
    searchWrapper.appendChild(searchInput);

    center.appendChild(colsSegment);
    center.appendChild(heightSegment);
    center.appendChild(searchWrapper);

    const right = document.createElement('div');
    right.className = 'topbar-right';

    const runningBadge = document.createElement('div');
    runningBadge.className = 'running-agents-badge';

    const dot = document.createElement('div');
    dot.className = 'pulse-dot';

    this.runningBadgeText = document.createElement('span');
    this.runningBadgeText.textContent = '0 RUNNING AGENTS';

    runningBadge.appendChild(dot);
    runningBadge.appendChild(this.runningBadgeText);

    const winControls = document.createElement('div');
    winControls.className = 'window-controls-group';

    const winMin = document.createElement('button');
    winMin.className = 'win-btn';
    winMin.textContent = '_';
    winMin.title = 'Minimize Window';

    const winMax = document.createElement('button');
    winMax.className = 'win-btn';
    winMax.textContent = '□';
    winMax.title = 'Maximize Window';

    const winClose = document.createElement('button');
    winClose.className = 'win-btn win-close';
    winClose.textContent = '✕';
    winClose.title = 'Close App';

    winControls.appendChild(winMin);
    winControls.appendChild(winMax);
    winControls.appendChild(winClose);

    const settingsBtn = document.createElement('button');
    settingsBtn.className = 'btn-icon';
    settingsBtn.innerHTML = '⚙';
    settingsBtn.title = 'Open Settings (Ctrl+,)';
    settingsBtn.onclick = () => this.callbacks.onOpenSettings?.();

    right.appendChild(runningBadge);
    right.appendChild(settingsBtn);
    right.appendChild(winControls);

    this.element.appendChild(left);
    this.element.appendChild(center);
    this.element.appendChild(right);

    container.appendChild(this.element);
  }

  /**
   * Updates column button highlight.
   */
  public setColumns(cols: GridColumns): void {
    this.currentCols = cols;
    this.segmentButtons.forEach((btn, val) => {
      btn.classList.toggle('active', val === cols);
    });
  }

  /**
   * Updates live active running agent counter.
   */
  public setRunningCount(count: number): void {
    this.runningBadgeText.textContent = `${count} RUNNING AGENT${count === 1 ? '' : 'S'}`;
  }
}
