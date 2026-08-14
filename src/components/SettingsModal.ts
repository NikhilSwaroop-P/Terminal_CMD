import { ThemeService } from '../services/ThemeService';
import { THEME_CATALOG } from '../themes/themeCatalog';
import { CursorStyleType } from '../themes/types';

/**
 * Interactive settings modal supporting whole-application themes and terminal preferences.
 */
export class SettingsModal {
  private backdropElement: HTMLElement;
  private cardElement: HTMLElement;
  private bodyElement: HTMLElement;
  private activeTab: 'themes' | 'terminal' = 'themes';
  private themeService: ThemeService;
  private isVisible = false;

  constructor() {
    this.themeService = ThemeService.getInstance();

    this.backdropElement = document.createElement('div');
    this.backdropElement.className = 'settings-modal-backdrop';
    this.backdropElement.style.display = 'none';

    this.cardElement = document.createElement('div');
    this.cardElement.className = 'settings-modal-card';

    const header = document.createElement('div');
    header.className = 'settings-modal-header';

    const title = document.createElement('div');
    title.className = 'settings-modal-title';
    title.innerHTML = '<span>⚙</span><span>TermCMD Settings</span>';

    const closeBtn = document.createElement('button');
    closeBtn.className = 'settings-close-btn';
    closeBtn.textContent = '✕';
    closeBtn.title = 'Close Settings (Esc)';
    closeBtn.onclick = () => this.hide();

    header.appendChild(title);
    header.appendChild(closeBtn);

    const navTabs = document.createElement('div');
    navTabs.className = 'settings-nav-tabs';

    const tabThemes = document.createElement('button');
    tabThemes.className = 'settings-tab-btn active';
    tabThemes.textContent = '🎨 Themes & Colors';
    tabThemes.onclick = () => {
      this.activeTab = 'themes';
      tabThemes.classList.add('active');
      tabTerminal.classList.remove('active');
      this.renderBody();
    };

    const tabTerminal = document.createElement('button');
    tabTerminal.className = 'settings-tab-btn';
    tabTerminal.textContent = '⌨ Terminal & Cursor';
    tabTerminal.onclick = () => {
      this.activeTab = 'terminal';
      tabTerminal.classList.add('active');
      tabThemes.classList.remove('active');
      this.renderBody();
    };

    navTabs.appendChild(tabThemes);
    navTabs.appendChild(tabTerminal);

    this.bodyElement = document.createElement('div');
    this.bodyElement.className = 'settings-modal-body';

    this.cardElement.appendChild(header);
    this.cardElement.appendChild(navTabs);
    this.cardElement.appendChild(this.bodyElement);
    this.backdropElement.appendChild(this.cardElement);

    this.backdropElement.onclick = (e) => {
      if (e.target === this.backdropElement) {
        this.hide();
      }
    };

    window.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && this.isVisible) {
        this.hide();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === ',') {
        e.preventDefault();
        this.toggle();
      }
    });

    document.body.appendChild(this.backdropElement);
    this.renderBody();
  }

  private renderBody(): void {
    this.bodyElement.innerHTML = '';

    if (this.activeTab === 'themes') {
      this.renderThemesTab();
    } else {
      this.renderTerminalTab();
    }
  }

  private renderThemesTab(): void {
    this.bodyElement.innerHTML = '';
    const currentTheme = this.themeService.getTheme();

    const title = document.createElement('div');
    title.className = 'settings-section-title';
    title.textContent = 'Application & Terminal Color Themes';
    this.bodyElement.appendChild(title);

    const grid = document.createElement('div');
    grid.className = 'theme-grid-container';

    THEME_CATALOG.forEach((theme) => {
      const card = document.createElement('div');
      card.className = `theme-card ${theme.id === currentTheme.id ? 'active' : ''}`;

      const cardHeader = document.createElement('div');
      cardHeader.className = 'theme-card-header';

      const cardTitle = document.createElement('span');
      cardTitle.className = 'theme-card-title';
      cardTitle.textContent = theme.name;
      cardHeader.appendChild(cardTitle);

      if (theme.id === currentTheme.id) {
        const badge = document.createElement('span');
        badge.className = 'theme-card-badge';
        badge.textContent = 'ACTIVE';
        cardHeader.appendChild(badge);
      }

      const cardDesc = document.createElement('div');
      cardDesc.className = 'theme-card-desc';
      cardDesc.textContent = theme.description;

      const swatches = document.createElement('div');
      swatches.className = 'theme-swatches';

      const colors = [
        theme.app.bgCard,
        theme.app.accentCyan,
        theme.app.accentEmerald,
        theme.app.accentAmber,
        theme.app.accentCoral
      ];

      colors.forEach((color) => {
        const dot = document.createElement('div');
        dot.className = 'theme-swatch-dot';
        dot.style.backgroundColor = color;
        swatches.appendChild(dot);
      });

      card.appendChild(cardHeader);
      card.appendChild(cardDesc);
      card.appendChild(swatches);

      card.onclick = () => {
        this.themeService.setTheme(theme.id);
        this.renderThemesTab();
      };

      grid.appendChild(card);
    });

    this.bodyElement.appendChild(grid);
  }

  private renderTerminalTab(): void {
    this.bodyElement.innerHTML = '';
    const settings = this.themeService.getSettings();

    const title = document.createElement('div');
    title.className = 'settings-section-title';
    title.textContent = 'Terminal Typography & Cursor';
    this.bodyElement.appendChild(title);

    const fontSizeRow = document.createElement('div');
    fontSizeRow.className = 'settings-control-row';
    fontSizeRow.innerHTML = `
      <div class="settings-control-label">
        <span class="settings-label-text">Font Size</span>
        <span class="settings-sub-text">Terminal text rendering size in pixels</span>
      </div>
    `;
    const fontSelect = document.createElement('select');
    fontSelect.className = 'settings-input-select';
    [11, 12, 13, 14, 15, 16, 18, 20].forEach((size) => {
      const opt = document.createElement('option');
      opt.value = String(size);
      opt.textContent = `${size}px`;
      if (size === settings.fontSize) opt.selected = true;
      fontSelect.appendChild(opt);
    });
    fontSelect.onchange = () => {
      this.themeService.setFontSize(Number(fontSelect.value));
    };
    fontSizeRow.appendChild(fontSelect);
    this.bodyElement.appendChild(fontSizeRow);

    const cursorStyleRow = document.createElement('div');
    cursorStyleRow.className = 'settings-control-row';
    cursorStyleRow.innerHTML = `
      <div class="settings-control-label">
        <span class="settings-label-text">Cursor Style</span>
        <span class="settings-sub-text">Default cursor shape (Line / Block / Underline)</span>
      </div>
    `;
    const cursorSelect = document.createElement('select');
    cursorSelect.className = 'settings-input-select';
    [
      { id: 'bar', name: 'Line (Bar)' },
      { id: 'block', name: 'Block' },
      { id: 'underline', name: 'Underline' }
    ].forEach((optData) => {
      const opt = document.createElement('option');
      opt.value = optData.id;
      opt.textContent = optData.name;
      if (optData.id === settings.cursorStyle) opt.selected = true;
      cursorSelect.appendChild(opt);
    });
    cursorSelect.onchange = () => {
      this.themeService.setCursorStyle(cursorSelect.value as CursorStyleType);
    };
    cursorStyleRow.appendChild(cursorSelect);
    this.bodyElement.appendChild(cursorStyleRow);

    const blinkRow = document.createElement('div');
    blinkRow.className = 'settings-control-row';
    blinkRow.innerHTML = `
      <div class="settings-control-label">
        <span class="settings-label-text">Cursor Blinking</span>
        <span class="settings-sub-text">Animate cursor pulse when focused</span>
      </div>
    `;
    const blinkToggle = document.createElement('input');
    blinkToggle.type = 'checkbox';
    blinkToggle.className = 'settings-toggle-checkbox';
    blinkToggle.checked = settings.cursorBlink;
    blinkToggle.onchange = () => {
      this.themeService.setCursorBlink(blinkToggle.checked);
    };
    blinkRow.appendChild(blinkToggle);
    this.bodyElement.appendChild(blinkRow);

    const trailRow = document.createElement('div');
    trailRow.className = 'settings-control-row';
    trailRow.innerHTML = `
      <div class="settings-control-label">
        <span class="settings-label-text">Smooth Cursor Trailing</span>
        <span class="settings-sub-text">Spring-damped motion blur trailing across lines</span>
      </div>
    `;
    const trailToggle = document.createElement('input');
    trailToggle.type = 'checkbox';
    trailToggle.className = 'settings-toggle-checkbox';
    trailToggle.checked = settings.cursorTrail;
    trailToggle.onchange = () => {
      this.themeService.setCursorTrail(trailToggle.checked);
    };
    trailRow.appendChild(trailToggle);
    this.bodyElement.appendChild(trailRow);
  }

  /**
   * Shows the settings modal.
   */
  public show(): void {
    this.isVisible = true;
    this.backdropElement.style.display = 'flex';
    this.renderBody();
  }

  /**
   * Hides the settings modal.
   */
  public hide(): void {
    this.isVisible = false;
    this.backdropElement.style.display = 'none';
  }

  /**
   * Toggles visibility of settings modal.
   */
  public toggle(): void {
    if (this.isVisible) {
      this.hide();
    } else {
      this.show();
    }
  }
}
