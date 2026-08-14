import { SearchAddon } from '@xterm/addon-search';

/**
 * In-tile floating search bar widget integrated with @xterm/addon-search.
 */
export class SearchOverlay {
  private element: HTMLElement;
  private inputField: HTMLInputElement;
  private matchCountSpan: HTMLSpanElement;
  private searchAddon: SearchAddon;
  private isVisible = false;
  private isCaseSensitive = false;
  private isRegex = false;

  constructor(searchAddon: SearchAddon) {
    this.searchAddon = searchAddon;

    this.element = document.createElement('div');
    this.element.className = 'search-overlay-container';
    this.element.style.display = 'none';

    this.inputField = document.createElement('input');
    this.inputField.type = 'text';
    this.inputField.className = 'search-input-field';
    this.inputField.placeholder = 'Find in buffer...';

    this.matchCountSpan = document.createElement('span');
    this.matchCountSpan.className = 'search-match-count';
    this.matchCountSpan.textContent = '';

    const btnCase = document.createElement('button');
    btnCase.className = 'search-btn';
    btnCase.textContent = 'Aa';
    btnCase.title = 'Match Case';
    btnCase.onclick = () => {
      this.isCaseSensitive = !this.isCaseSensitive;
      btnCase.classList.toggle('active-toggle', this.isCaseSensitive);
      this.triggerSearch();
    };

    const btnRegex = document.createElement('button');
    btnRegex.className = 'search-btn';
    btnRegex.textContent = '.*';
    btnRegex.title = 'Use Regular Expression';
    btnRegex.onclick = () => {
      this.isRegex = !this.isRegex;
      btnRegex.classList.toggle('active-toggle', this.isRegex);
      this.triggerSearch();
    };

    const btnPrev = document.createElement('button');
    btnPrev.className = 'search-btn';
    btnPrev.textContent = '▲';
    btnPrev.title = 'Previous Match (Shift+Enter)';
    btnPrev.onclick = () => this.findPrevious();

    const btnNext = document.createElement('button');
    btnNext.className = 'search-btn';
    btnNext.textContent = '▼';
    btnNext.title = 'Next Match (Enter)';
    btnNext.onclick = () => this.findNext();

    const btnClose = document.createElement('button');
    btnClose.className = 'search-btn';
    btnClose.textContent = '✕';
    btnClose.title = 'Close (Escape)';
    btnClose.onclick = () => this.hide();

    this.inputField.oninput = () => this.triggerSearch();
    this.inputField.onkeydown = (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (e.shiftKey) {
          this.findPrevious();
        } else {
          this.findNext();
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        this.hide();
      }
    };

    this.element.appendChild(this.inputField);
    this.element.appendChild(this.matchCountSpan);
    this.element.appendChild(btnCase);
    this.element.appendChild(btnRegex);
    this.element.appendChild(btnPrev);
    this.element.appendChild(btnNext);
    this.element.appendChild(btnClose);
  }

  /**
   * Returns root DOM element for search widget.
   */
  public getElement(): HTMLElement {
    return this.element;
  }

  /**
   * Displays the search overlay and focuses input.
   */
  public show(): void {
    this.isVisible = true;
    this.element.style.display = 'flex';
    this.inputField.focus();
    this.inputField.select();
    if (this.inputField.value) {
      this.triggerSearch();
    }
  }

  /**
   * Hides the search overlay and clears active search decorations.
   */
  public hide(): void {
    this.isVisible = false;
    this.element.style.display = 'none';
    this.searchAddon.clearDecorations();
    this.matchCountSpan.textContent = '';
  }

  /**
   * Toggles visibility.
   */
  public toggle(): void {
    if (this.isVisible) {
      this.hide();
    } else {
      this.show();
    }
  }

  private triggerSearch(): void {
    const term = this.inputField.value;
    if (!term) {
      this.searchAddon.clearDecorations();
      this.matchCountSpan.textContent = '';
      return;
    }
    this.searchAddon.findNext(term, {
      caseSensitive: this.isCaseSensitive,
      regex: this.isRegex,
      incremental: true
    });
  }

  private findNext(): void {
    const term = this.inputField.value;
    if (!term) return;
    this.searchAddon.findNext(term, {
      caseSensitive: this.isCaseSensitive,
      regex: this.isRegex
    });
  }

  private findPrevious(): void {
    const term = this.inputField.value;
    if (!term) return;
    this.searchAddon.findPrevious(term, {
      caseSensitive: this.isCaseSensitive,
      regex: this.isRegex
    });
  }
}
