/**
 * Clipboard service enabling Kitty-style copy-on-select functionality.
 */
export class ClipboardService {
  /**
   * Copies provided text to system clipboard.
   */
  public static async copyText(text: string): Promise<boolean> {
    if (!text || text.trim().length === 0) {
      return false;
    }
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(text);
        return true;
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.focus();
        textarea.select();
        const success = document.execCommand('copy');
        document.body.removeChild(textarea);
        return success;
      }
    } catch {
      return false;
    }
  }
}
