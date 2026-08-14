/**
 * Full-duplex WebSocket client for interactive terminal streaming.
 */
export class PtyWebSocket {
  private terminalId: string;
  private wsUrl: string;
  private token: string;
  private socket: WebSocket | null = null;
  private isExplicitlyClosed = false;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectTimer: number | null = null;
  private pendingQueue: (string | Uint8Array)[] = [];

  private dataListeners: ((data: Uint8Array | string) => void)[] = [];
  private openListeners: (() => void)[] = [];
  private closeListeners: (() => void)[] = [];
  private errorListeners: ((err: Event) => void)[] = [];

  constructor(terminalId: string, customWsUrl?: string, token?: string) {
    this.terminalId = terminalId;
    this.token = token || (typeof localStorage !== 'undefined' ? localStorage.getItem('termcmd_token') || '' : '');

    if (customWsUrl) {
      this.wsUrl = customWsUrl;
    } else {
      this.wsUrl = `ws://127.0.0.1:7890/api/v1/terminals/${terminalId}/ws`;
    }
  }

  /**
   * Returns associated terminal session ID.
   */
  public getTerminalId(): string {
    return this.terminalId;
  }

  /**
   * Connects to the backend WebSocket endpoint.
   */
  public async connect(): Promise<void> {
    if (this.socket && (this.socket.readyState === WebSocket.OPEN || this.socket.readyState === WebSocket.CONNECTING)) {
      return;
    }

    if (!this.token) {
      if (typeof localStorage !== 'undefined') {
        this.token = localStorage.getItem('termcmd_token') || '';
      }
      if (!this.token) {
        try {
          const res = await fetch('http://127.0.0.1:7890/__token');
          if (res.ok) {
            const data = await res.json();
            if (data.token) {
              this.token = data.token;
              if (typeof localStorage !== 'undefined') {
                localStorage.setItem('termcmd_token', data.token);
              }
            }
          }
        } catch {}
      }
    }

    this.isExplicitlyClosed = false;
    const authUrl = this.token ? `${this.wsUrl}?token=${encodeURIComponent(this.token)}` : this.wsUrl;
    this.socket = new WebSocket(authUrl);
    this.socket.binaryType = 'arraybuffer';

    this.socket.onopen = () => {
      this.reconnectAttempts = 0;
      this.flushPendingQueue();
      this.openListeners.forEach(listener => listener());
    };

    this.socket.onmessage = (event: MessageEvent) => {
      if (event.data instanceof ArrayBuffer) {
        const bytes = new Uint8Array(event.data);
        this.dataListeners.forEach(listener => listener(bytes));
      } else if (typeof event.data === 'string') {
        this.dataListeners.forEach(listener => listener(event.data));
      }
    };

    this.socket.onclose = () => {
      this.closeListeners.forEach(listener => listener());
      if (!this.isExplicitlyClosed) {
        this.scheduleReconnect();
      }
    };

    this.socket.onerror = (err: Event) => {
      this.errorListeners.forEach(listener => listener(err));
    };
  }

  /**
   * Transmits input string or binary data to the backend PTY.
   */
  public send(data: string | Uint8Array): void {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(data);
    } else {
      this.pendingQueue.push(data);
    }
  }

  private flushPendingQueue(): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      return;
    }
    while (this.pendingQueue.length > 0) {
      const item = this.pendingQueue.shift();
      if (item) {
        this.socket.send(item);
      }
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      return;
    }
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 8000);
    this.reconnectAttempts++;
    this.reconnectTimer = window.setTimeout(() => {
      this.connect();
    }, delay);
  }

  /**
   * Registers a callback for receiving stdout/stderr terminal bytes.
   */
  public onData(listener: (data: Uint8Array | string) => void): () => void {
    this.dataListeners.push(listener);
    return () => {
      this.dataListeners = this.dataListeners.filter(l => l !== listener);
    };
  }

  /**
   * Registers a callback for connection open.
   */
  public onOpen(listener: () => void): () => void {
    this.openListeners.push(listener);
    return () => {
      this.openListeners = this.openListeners.filter(l => l !== listener);
    };
  }

  /**
   * Registers a callback for connection close.
   */
  public onClose(listener: () => void): () => void {
    this.closeListeners.push(listener);
    return () => {
      this.closeListeners = this.closeListeners.filter(l => l !== listener);
    };
  }

  /**
   * Registers a callback for connection error.
   */
  public onError(listener: (err: Event) => void): () => void {
    this.errorListeners.push(listener);
    return () => {
      this.errorListeners = this.errorListeners.filter(l => l !== listener);
    };
  }

  /**
   * Explicitly closes and cleans up the WebSocket connection.
   */
  public close(): void {
    this.isExplicitlyClosed = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
    this.dataListeners = [];
    this.openListeners = [];
    this.closeListeners = [];
    this.errorListeners = [];
    this.pendingQueue = [];
  }
}
