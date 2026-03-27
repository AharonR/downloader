<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface CookieStatus {
    has_cookies: boolean;
    domain_count: number;
    domains: string[];
  }

  interface CookieImportResult {
    domain_count: number;
    cookie_count: number;
    warnings: string[];
    storage_path: string;
  }

  let expanded = $state(false);
  let cookieStatus = $state<CookieStatus | null>(null);
  let pasteText = $state('');
  let importing = $state(false);
  let feedback = $state<{ type: 'success' | 'error'; message: string } | null>(null);

  async function loadStatus() {
    try {
      cookieStatus = await invoke<CookieStatus>('get_cookie_status');
    } catch {
      cookieStatus = null;
    }
  }

  onMount(() => {
    loadStatus();
  });

  async function handlePasteImport() {
    if (!pasteText.trim() || importing) return;
    importing = true;
    feedback = null;
    try {
      const result = await invoke<CookieImportResult>('import_cookies', { input: pasteText });
      feedback = {
        type: 'success',
        message: `Saved ${result.cookie_count} cookie${result.cookie_count !== 1 ? 's' : ''} for ${result.domain_count} domain${result.domain_count !== 1 ? 's' : ''}.`,
      };
      pasteText = '';
      await loadStatus();
    } catch (err) {
      feedback = { type: 'error', message: typeof err === 'string' ? err : String(err) };
    } finally {
      importing = false;
    }
  }

  async function handleFileImport() {
    if (importing) return;
    importing = true;
    feedback = null;
    try {
      const result = await invoke<CookieImportResult>('import_cookies_from_file');
      feedback = {
        type: 'success',
        message: `Saved ${result.cookie_count} cookie${result.cookie_count !== 1 ? 's' : ''} for ${result.domain_count} domain${result.domain_count !== 1 ? 's' : ''}.`,
      };
      await loadStatus();
    } catch (err) {
      const msg = typeof err === 'string' ? err : String(err);
      if (msg.includes('No file selected')) {
        // User cancelled picker — not an error worth showing.
        feedback = null;
      } else {
        feedback = { type: 'error', message: msg };
      }
    } finally {
      importing = false;
    }
  }

  async function handleClear() {
    if (importing) return;
    feedback = null;
    try {
      await invoke<boolean>('clear_cookies');
      feedback = { type: 'success', message: 'Cookies cleared.' };
      await loadStatus();
    } catch (err) {
      feedback = { type: 'error', message: typeof err === 'string' ? err : String(err) };
    }
  }
</script>

<div class="auth-panel">
  <button
    type="button"
    class="auth-toggle"
    onclick={() => (expanded = !expanded)}
    aria-expanded={expanded}
  >
    <span class="auth-toggle-label">
      Publisher authentication
      {#if cookieStatus?.has_cookies}
        <span class="auth-badge">{cookieStatus.domain_count} domain{cookieStatus.domain_count !== 1 ? 's' : ''}</span>
      {/if}
    </span>
    <span class="auth-toggle-chevron" class:auth-toggle-chevron--open={expanded}></span>
  </button>

  {#if expanded}
    <div class="auth-body">
      {#if cookieStatus?.has_cookies}
        <div class="auth-status">
          <p class="auth-status-text">
            Cookies saved for: {cookieStatus.domains.join(', ')}
          </p>
          <button type="button" class="auth-clear-btn" onclick={handleClear} disabled={importing}>
            Clear cookies
          </button>
        </div>
      {/if}

      <div class="auth-instructions">
        <p>To download from paywalled publishers, export cookies from your browser after logging in:</p>
        <ol>
          <li>Log in to the publisher site in your browser.</li>
          <li>Use a browser extension (e.g. "Get cookies.txt LOCALLY") to export cookies.</li>
          <li>Paste the exported text below, or import the file directly.</li>
        </ol>
      </div>

      <textarea
        class="auth-textarea"
        bind:value={pasteText}
        placeholder="Paste Netscape or JSON cookie data here..."
        rows={4}
        disabled={importing}
      ></textarea>

      <div class="auth-actions">
        <button
          type="button"
          class="auth-import-btn"
          onclick={handlePasteImport}
          disabled={!pasteText.trim() || importing}
        >
          {importing ? 'Importing...' : 'Import from paste'}
        </button>
        <button
          type="button"
          class="auth-file-btn"
          onclick={handleFileImport}
          disabled={importing}
        >
          Import cookies.txt file
        </button>
      </div>

      {#if feedback}
        <p class="auth-feedback" class:auth-feedback--success={feedback.type === 'success'} class:auth-feedback--error={feedback.type === 'error'} role="alert">
          {feedback.message}
        </p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .auth-panel {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-field);
    background: rgba(246, 240, 228, 0.5);
  }

  .auth-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.65rem 0.9rem;
    border: none;
    border-radius: var(--radius-field);
    background: transparent;
    color: var(--fg-strong);
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .auth-toggle:hover {
    background: rgba(224, 234, 223, 0.5);
  }

  .auth-toggle-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .auth-badge {
    display: inline-flex;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-primary);
    font-size: 0.74rem;
    font-weight: 600;
  }

  .auth-toggle-chevron {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    border-right: 2px solid var(--fg-muted);
    border-bottom: 2px solid var(--fg-muted);
    transform: rotate(45deg);
    transition: transform 0.2s ease;
  }

  .auth-toggle-chevron--open {
    transform: rotate(-135deg);
  }

  .auth-body {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0 0.9rem 0.9rem;
  }

  .auth-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.55rem 0.75rem;
    border-radius: 10px;
    background: rgba(224, 234, 223, 0.7);
    border: 1px solid rgba(53, 91, 70, 0.15);
  }

  .auth-status-text {
    margin: 0;
    color: var(--accent-primary);
    font-size: 0.82rem;
    font-weight: 500;
  }

  .auth-clear-btn {
    background: transparent;
    color: var(--state-error);
    border: 1px solid rgba(143, 68, 54, 0.3);
    border-radius: 999px;
    padding: 0.3rem 0.65rem;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s ease;
  }

  .auth-clear-btn:hover:not(:disabled) {
    background: rgba(246, 228, 223, 0.75);
  }

  .auth-clear-btn:disabled {
    color: var(--fg-muted);
    cursor: not-allowed;
  }

  .auth-instructions {
    color: var(--fg-muted);
    font-size: 0.82rem;
    line-height: 1.5;
  }

  .auth-instructions p {
    margin: 0 0 0.35rem;
  }

  .auth-instructions ol {
    margin: 0;
    padding-left: 1.3rem;
  }

  .auth-instructions li {
    margin-bottom: 0.15rem;
  }

  .auth-textarea {
    width: 100%;
    padding: 0.65rem 0.8rem;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-field);
    background: rgba(255, 255, 255, 0.62);
    color: var(--fg-strong);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    line-height: 1.5;
    resize: vertical;
    box-sizing: border-box;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .auth-textarea:focus {
    border-color: rgba(53, 91, 70, 0.55);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.6), 0 0 0 4px rgba(53, 91, 70, 0.09);
    background: rgba(255, 255, 255, 0.82);
  }

  .auth-textarea:disabled {
    background: rgba(240, 236, 226, 0.82);
    color: var(--fg-muted);
  }

  .auth-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .auth-import-btn,
  .auth-file-btn {
    background: transparent;
    color: var(--accent-primary);
    border: 1px solid rgba(53, 91, 70, 0.35);
    border-radius: 999px;
    padding: 0.45rem 0.85rem;
    font-size: 0.84rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.18s ease, border-color 0.18s ease;
  }

  .auth-import-btn:hover:not(:disabled),
  .auth-file-btn:hover:not(:disabled) {
    background: rgba(224, 234, 223, 0.78);
    border-color: rgba(53, 91, 70, 0.5);
  }

  .auth-import-btn:disabled,
  .auth-file-btn:disabled {
    color: var(--fg-muted);
    border-color: rgba(180, 175, 157, 0.7);
    cursor: not-allowed;
  }

  .auth-feedback {
    margin: 0;
    padding: 0.5rem 0.75rem;
    border-radius: 10px;
    font-size: 0.82rem;
    font-weight: 500;
  }

  .auth-feedback--success {
    background: rgba(224, 234, 223, 0.7);
    color: var(--state-success);
    border: 1px solid rgba(63, 111, 81, 0.15);
  }

  .auth-feedback--error {
    background: rgba(246, 228, 223, 0.6);
    color: var(--state-error);
    border: 1px solid rgba(143, 68, 54, 0.15);
    white-space: pre-line;
  }
</style>
