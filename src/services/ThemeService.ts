import { AppTheme, AppSettings, CursorStyleType } from '../themes/types';
import { THEME_CATALOG } from '../themes/themeCatalog';

export type ThemeChangeListener = (theme: AppTheme, settings: AppSettings) => void;

/**
 * Singleton theme and terminal preference manager with real-time DOM injection and persistence.
 */
export class ThemeService {
  private static instance: ThemeService | null = null;
  private currentTheme: AppTheme;
  private settings: AppSettings;
  private listeners: ThemeChangeListener[] = [];

  private constructor() {
    this.settings = this.loadSettings();
    this.currentTheme =
      THEME_CATALOG.find((t) => t.id === this.settings.themeId) || THEME_CATALOG[0];
    this.applyTheme(this.currentTheme);
  }

  public static getInstance(): ThemeService {
    if (!ThemeService.instance) {
      ThemeService.instance = new ThemeService();
    }
    return ThemeService.instance;
  }

  private loadSettings(): AppSettings {
    const defaultSettings: AppSettings = {
      themeId: 'default',
      fontSize: 13,
      lineHeight: 1.25,
      cursorStyle: 'bar',
      cursorBlink: true,
      cursorTrail: true
    };

    try {
      const stored = localStorage.getItem('termcmd_settings');
      if (stored) {
        return { ...defaultSettings, ...JSON.parse(stored) };
      }
    } catch {}

    return defaultSettings;
  }

  private saveSettings(): void {
    try {
      localStorage.setItem('termcmd_settings', JSON.stringify(this.settings));
    } catch {}
  }

  /**
   * Returns current active theme.
   */
  public getTheme(): AppTheme {
    return this.currentTheme;
  }

  /**
   * Returns current application settings.
   */
  public getSettings(): AppSettings {
    return { ...this.settings };
  }

  /**
   * Sets active theme by ID and updates DOM styles and listeners.
   */
  public setTheme(themeId: string): void {
    const theme = THEME_CATALOG.find((t) => t.id === themeId);
    if (!theme) return;

    this.currentTheme = theme;
    this.settings.themeId = theme.id;
    this.applyTheme(theme);
    this.saveSettings();
    this.notifyListeners();
  }

  /**
   * Updates font size in pixels.
   */
  public setFontSize(size: number): void {
    this.settings.fontSize = Math.max(10, Math.min(24, size));
    this.saveSettings();
    this.notifyListeners();
  }

  /**
   * Updates cursor style ('bar' | 'block' | 'underline').
   */
  public setCursorStyle(style: CursorStyleType): void {
    this.settings.cursorStyle = style;
    this.saveSettings();
    this.notifyListeners();
  }

  /**
   * Toggles cursor blinking.
   */
  public setCursorBlink(blink: boolean): void {
    this.settings.cursorBlink = blink;
    this.saveSettings();
    this.notifyListeners();
  }

  /**
   * Toggles spring cursor trailing shader.
   */
  public setCursorTrail(enabled: boolean): void {
    this.settings.cursorTrail = enabled;
    this.saveSettings();
    this.notifyListeners();
  }

  /**
   * Applies CSS variables to document root element.
   */
  private applyTheme(theme: AppTheme): void {
    const root = document.documentElement;
    const { app } = theme;

    root.style.setProperty('--bg-app', app.bgApp);
    root.style.setProperty('--bg-header', app.bgHeader);
    root.style.setProperty('--bg-sidebar', app.bgSidebar);
    root.style.setProperty('--bg-dock', app.bgDock);
    root.style.setProperty('--bg-card', app.bgCard);
    root.style.setProperty(
      '--bg-card-gradient',
      `linear-gradient(180deg, ${app.bgCard} 0%, ${app.bgApp} 100%)`
    );
    root.style.setProperty('--bg-tile-header', app.bgTileHeader);
    root.style.setProperty('--bg-tile-header-active', app.bgTileHeaderActive);
    root.style.setProperty('--bg-input', app.bgInput);
    root.style.setProperty('--bg-button', app.bgButton);
    root.style.setProperty('--bg-button-hover', app.bgButtonHover);

    root.style.setProperty('--border-subtle', app.borderSubtle);
    root.style.setProperty('--border-default', app.borderDefault);
    root.style.setProperty('--border-active', app.borderActive);
    root.style.setProperty('--border-focus', app.borderFocus);
    root.style.setProperty('--border-streaming', app.borderStreaming);
    root.style.setProperty('--border-running', app.borderRunning);
    root.style.setProperty('--border-idle', app.borderIdle);
    root.style.setProperty('--border-terminated', app.borderTerminated);

    root.style.setProperty('--accent-cyan', app.accentCyan);
    root.style.setProperty('--accent-cyan-glow', app.accentCyanGlow);
    root.style.setProperty('--accent-emerald', app.accentEmerald);
    root.style.setProperty('--accent-emerald-glow', app.accentEmeraldGlow);
    root.style.setProperty('--accent-amber', app.accentAmber);
    root.style.setProperty('--accent-amber-glow', app.accentAmberGlow);
    root.style.setProperty('--accent-coral', app.accentCoral);
    root.style.setProperty('--accent-coral-glow', app.accentCoralGlow);

    root.style.setProperty('--text-primary', app.textPrimary);
    root.style.setProperty('--text-secondary', app.textSecondary);
    root.style.setProperty('--text-muted', app.textMuted);
  }

  /**
   * Subscribes to theme and preference updates.
   */
  public subscribe(listener: ThemeChangeListener): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notifyListeners(): void {
    this.listeners.forEach((listener) => {
      listener(this.currentTheme, this.settings);
    });
  }
}
