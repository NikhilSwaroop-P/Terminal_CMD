import { ITheme } from '@xterm/xterm';

export type CursorStyleType = 'bar' | 'block' | 'underline';

export interface AppTheme {
  id: string;
  name: string;
  description: string;
  app: {
    bgApp: string;
    bgHeader: string;
    bgSidebar: string;
    bgDock: string;
    bgCard: string;
    bgTileHeader: string;
    bgTileHeaderActive: string;
    bgInput: string;
    bgButton: string;
    bgButtonHover: string;
    borderSubtle: string;
    borderDefault: string;
    borderActive: string;
    borderFocus: string;
    borderStreaming: string;
    borderRunning: string;
    borderIdle: string;
    borderTerminated: string;
    accentCyan: string;
    accentCyanGlow: string;
    accentEmerald: string;
    accentEmeraldGlow: string;
    accentAmber: string;
    accentAmberGlow: string;
    accentCoral: string;
    accentCoralGlow: string;
    textPrimary: string;
    textSecondary: string;
    textMuted: string;
  };
  terminal: ITheme;
}

export interface AppSettings {
  themeId: string;
  fontSize: number;
  lineHeight: number;
  cursorStyle: CursorStyleType;
  cursorBlink: boolean;
  cursorTrail: boolean;
}
