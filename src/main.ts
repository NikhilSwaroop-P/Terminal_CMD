import { defaultApiClient } from './services/ApiClient';
import { SortManager } from './services/SortManager';
import { KeybindingManager } from './services/KeybindingManager';
import { TopBar } from './components/TopBar';
import { Sidebar } from './components/Sidebar';
import { Canvas } from './components/Canvas';
import { DockTray } from './components/DockTray';
import { TerminalSessionInfo } from './types/terminal';

/**
 * Main application bootstrap for TermCMD Desktop Canvas.
 */
export async function bootstrapApp(): Promise<void> {
  const root = document.getElementById('app');
  if (!root) throw new Error('Missing #app container');

  const apiClient = defaultApiClient;
  const sortManager = new SortManager('running_priority');

  const topBar = new TopBar(root, {
    onToggleSidebar: () => sidebar.toggle(),
    onNewTerminal: async () => {
      try {
        const info = await apiClient.createTerminal();
        canvas.addTerminal(info);
      } catch {}
    },
    onSetColumns: (cols) => {
      canvas.setColumns(cols);
      dockTray.setCanvasInfo(`Tiling Canvas (${cols} cols) | Scrollable`);
    },
    onSetDefaultHeight: (h) => canvas.setDefaultHeight(h),
    onSearchQueryChange: (q) => sidebar.setSearchQuery(q)
  });

  const appBody = document.createElement('div');
  appBody.className = 'app-body';
  root.appendChild(appBody);

  const sidebar = new Sidebar(
    appBody,
    {
      onSelectTerminal: (id) => canvas.focusTerminal(id),
      onPinTopTerminal: (id) => {
        sortManager.pinToTop(id);
        canvas.pinAndScrollToTop(id);
      },
      onCloseTerminal: (id) => canvas.closeTerminal(id),
      onSortModeChange: (mode) => {
        sortManager.setSortMode(mode);
        const sessions = canvas.getSessionInfos();
        sidebar.updateSessions(sessions);
      }
    },
    sortManager
  );

  const canvasContainer = document.createElement('main');
  appBody.appendChild(canvasContainer);

  const canvas = new Canvas(
    canvasContainer,
    {
      onSessionCountChange: () => {
        const sessions = canvas.getSessionInfos();
        sidebar.updateSessions(sessions);
      },
      onActiveTerminalChange: (id) => {
        sidebar.setActiveTerminalId(id);
        if (id) {
          sortManager.recordActivity(id);
        }
      },
      onRunningCountChange: (count) => {
        topBar.setRunningCount(count);
      },
      onMinimizeTerminal: (session) => {
        dockTray.addMinimized(session);
      }
    },
    apiClient
  );

  const dockTray = new DockTray(root, {
    onRestoreTerminal: (id) => canvas.restoreTerminal(id)
  });

  const keybindings = new KeybindingManager({
    onNewTerminal: async () => {
      try {
        const info = await apiClient.createTerminal();
        canvas.addTerminal(info);
      } catch {}
    },
    onSetColumns: (cols) => {
      topBar.setColumns(cols);
      canvas.setColumns(cols);
      dockTray.setCanvasInfo(`Tiling Canvas (${cols} cols) | Scrollable`);
    },
    onCloseFocused: () => {
      const activeId = canvas.getActiveTerminalId();
      if (activeId) canvas.closeTerminal(activeId);
    },
    onMinimizeFocused: () => {
      const activeId = canvas.getActiveTerminalId();
      if (activeId) canvas.minimizeTerminal(activeId);
    },
    onCycleFocusNext: () => canvas.cycleFocus(true),
    onCycleFocusPrev: () => canvas.cycleFocus(false),
    onToggleSearch: () => canvas.toggleSearchOnFocused(),
    onToggleSidebar: () => sidebar.toggle()
  });

  keybindings.attach();

  try {
    const existing = await apiClient.listTerminals();
    if (existing.length > 0) {
      existing.forEach((term: TerminalSessionInfo) => canvas.addTerminal(term));
    } else {
      const initial = await apiClient.createTerminal({ title: 'Terminal 1' });
      canvas.addTerminal(initial);
    }
  } catch {
    const demoInfo: TerminalSessionInfo = {
      id: 'term_default',
      title: 'Terminal 1',
      cwd: '/workspace',
      pid: 1001,
      state: 'IDLE',
      createdAt: new Date().toISOString()
    };
    canvas.addTerminal(demoInfo);
  }

  setInterval(async () => {
    try {
      const list = await apiClient.listTerminals();
      canvas.syncTerminals(list);
      sidebar.updateSessions(canvas.getSessionInfos());
    } catch {}
  }, 2000);
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => bootstrapApp());
} else {
  bootstrapApp();
}
