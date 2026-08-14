import {
  TerminalCreateOptions,
  TerminalSessionInfo,
  SSEExecEvent
} from '../types/terminal';

/**
 * REST and SSE client for the local TermCMD Agent API server.
 */
export class ApiClient {
  private baseUrl: string;
  private token: string;

  constructor(baseUrl?: string, token?: string) {
    if (baseUrl) {
      this.baseUrl = baseUrl;
    } else if (typeof window !== 'undefined' && window.location.port === '5173') {
      this.baseUrl = '';
    } else {
      this.baseUrl = 'http://127.0.0.1:7890';
    }
    this.token = token || this.discoverToken();
  }

  /**
   * Discovers active bearer token from query params, local storage, or environment.
   */
  public discoverToken(): string {
    if (typeof window !== 'undefined' && window.location) {
      const urlParams = new URLSearchParams(window.location.search);
      const queryToken = urlParams.get('token');
      if (queryToken) {
        if (typeof localStorage !== 'undefined') {
          localStorage.setItem('termcmd_token', queryToken);
        }
        return queryToken;
      }
    }
    if (typeof localStorage !== 'undefined') {
      const stored = localStorage.getItem('termcmd_token');
      if (stored) {
        return stored;
      }
    }
    return '';
  }

  /**
   * Updates the bearer token.
   */
  public setToken(token: string): void {
    this.token = token;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('termcmd_token', token);
    }
  }

  /**
   * Returns current token.
   */
  public getToken(): string {
    return this.token;
  }

  /**
   * Returns configured base URL.
   */
  public getBaseUrl(): string {
    return this.baseUrl;
  }

  private getHeaders(): HeadersInit {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json'
    };
    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }
    return headers;
  }

  /**
   * Spawns a new persistent terminal session.
   */
  public async createTerminal(options: TerminalCreateOptions = {}): Promise<TerminalSessionInfo> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals`, {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify(options)
    });
    if (!res.ok) {
      throw new Error(`Failed to create terminal: ${res.statusText}`);
    }
    return res.json();
  }

  /**
   * Fetches list of all active terminal sessions.
   */
  public async listTerminals(): Promise<TerminalSessionInfo[]> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals`, {
      method: 'GET',
      headers: this.getHeaders()
    });
    if (!res.ok) {
      throw new Error(`Failed to list terminals: ${res.statusText}`);
    }
    const data = await res.json();
    return data.terminals || [];
  }

  /**
   * Inspects detailed session info and scrollback buffer snapshot.
   */
  public async getTerminal(id: string): Promise<{ terminal: TerminalSessionInfo; buffer: string[] }> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals/${id}`, {
      method: 'GET',
      headers: this.getHeaders()
    });
    if (!res.ok) {
      throw new Error(`Failed to get terminal ${id}: ${res.statusText}`);
    }
    return res.json();
  }

  /**
   * Resizes terminal dimensions and dispatches SIGWINCH to backend PTY.
   */
  public async resizeTerminal(
    id: string,
    cols: number,
    rows: number
  ): Promise<{ resized: boolean; cols: number; rows: number }> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals/${id}/resize`, {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify({ cols, rows })
    });
    if (!res.ok) {
      throw new Error(`Failed to resize terminal ${id}: ${res.statusText}`);
    }
    return res.json();
  }

  /**
   * Sends raw keystroke inputs to the terminal's slave PTY.
   */
  public async sendInput(id: string, data: string): Promise<{ success: boolean }> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals/${id}/input`, {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify({ data })
    });
    if (!res.ok) {
      throw new Error(`Failed to send input to terminal ${id}: ${res.statusText}`);
    }
    return res.json();
  }

  /**
   * Sends a POSIX signal to the foreground process group.
   */
  public async killTerminal(id: string, signal = 'SIGINT'): Promise<{ signaled: boolean; signal: string }> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals/${id}/kill`, {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify({ signal })
    });
    if (!res.ok) {
      throw new Error(`Failed to kill terminal ${id}: ${res.statusText}`);
    }
    return res.json();
  }

  /**
   * Terminates and deletes a terminal session.
   */
  public async deleteTerminal(id: string): Promise<{ closed: boolean }> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals/${id}`, {
      method: 'DELETE',
      headers: this.getHeaders()
    });
    if (!res.ok) {
      throw new Error(`Failed to delete terminal ${id}: ${res.statusText}`);
    }
    return res.json();
  }

  /**
   * Executes a command and streams output via Server-Sent Events (SSE).
   */
  public async execTerminal(
    id: string,
    command: string,
    onEvent: (ev: SSEExecEvent) => void,
    stripAnsi = false,
    timeoutSeconds = 300
  ): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/v1/terminals/${id}/exec`, {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify({ command, stripAnsi, timeoutSeconds })
    });

    if (!res.ok) {
      throw new Error(`Failed to execute in terminal ${id}: ${res.statusText}`);
    }

    if (!res.body) {
      return;
    }

    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split('\n\n');
      buffer = lines.pop() || '';

      for (const block of lines) {
        const blockLines = block.split('\n');
        let eventType: SSEExecEvent['event'] = 'stdout';
        let eventData = '';

        for (const line of blockLines) {
          if (line.startsWith('event:')) {
            eventType = line.substring(6).trim() as SSEExecEvent['event'];
          } else if (line.startsWith('data:')) {
            eventData = line.substring(5).trim();
          }
        }

        onEvent({ event: eventType, data: eventData });
      }
    }
  }
}

export const defaultApiClient = new ApiClient();
