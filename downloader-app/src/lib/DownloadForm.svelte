<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onDestroy } from 'svelte';
  import StatusDisplay from './StatusDisplay.svelte';
  import ProgressDisplay from './ProgressDisplay.svelte';
  import CompletionSummary from './CompletionSummary.svelte';
  import ProjectSelector from './ProjectSelector.svelte';
  import AuthPanel from './AuthPanel.svelte';
  import type { ProgressPayload } from './ProgressDisplay.svelte';
  import type { DownloadSummary } from './CompletionSummary.svelte';

  let projectName = $state('');
  let inputText = $state('');
  let bibFiles = $state<string[]>([]);
  let bibPickerError = $state<string | null>(null);
  let status = $state<'idle' | 'downloading' | 'done' | 'error'>('idle');
  let message = $state('');
  let progressPayload = $state<ProgressPayload | null>(null);
  let summary = $state<DownloadSummary | null>(null);
  let cancelled = $state(false);
  let cancelRequested = $state(false);

  let unlisten: (() => void) | null = null;

  let isInputEmpty = $derived(inputText.trim() === '' && bibFiles.length === 0);
  let isDownloading = $derived(status === 'downloading');

  async function handleDownload(event: Event) {
    event.preventDefault();
    if (isInputEmpty || isDownloading) return;

    status = 'downloading';
    message = '';
    progressPayload = null;
    summary = null;
    cancelled = false;
    cancelRequested = false;
    bibPickerError = null;

    try {
      unlisten = await listen<ProgressPayload>('download://progress', (e) => {
        progressPayload = e.payload;
      });

      const inputs = inputText
        .split('\n')
        .map((s) => s.trim())
        .filter((s) => s.length > 0);

      const result = await invoke<DownloadSummary>('start_download_with_progress', {
        inputs,
        project: projectName || null,
        bibliography_paths: bibFiles,
      });

      summary = result;
      status = 'done';
      message = `Downloaded ${result.completed} file${result.completed !== 1 ? 's' : ''} to ${result.output_dir}`;
    } catch (err) {
      status = 'error';
      message = typeof err === 'string' ? err : String(err);
    } finally {
      unlisten?.();
      unlisten = null;
    }
  }

  async function handleCancel() {
    if (cancelRequested) return;
    cancelRequested = true;
    cancelled = true;
    try {
      await invoke('cancel_download');
    } catch {
      // Ignore cancel errors — engine will finish naturally.
    }
  }

  async function handleAddBibFiles() {
    bibPickerError = null;
    try {
      const paths = await invoke<string[]>('pick_bibliography_files');
      bibFiles = [...bibFiles, ...paths];
    } catch (err) {
      bibPickerError = typeof err === 'string' ? err : 'Could not open file picker';
    }
  }

  function removeBibFile(index: number) {
    bibFiles = bibFiles.filter((_, i) => i !== index);
  }

  function handleReset() {
    status = 'idle';
    message = '';
    progressPayload = null;
    summary = null;
    cancelled = false;
    cancelRequested = false;
    inputText = '';
    bibFiles = [];
    bibPickerError = null;
  }

  onDestroy(() => {
    unlisten?.();
  });
</script>

<div class="download-form-container">
  <section class="panel panel--intake">
    <div class="panel-header">
      <div>
        <p class="panel-kicker">Intake</p>
        <h2>Start with the sources you already trust.</h2>
      </div>
      <p class="panel-copy">Create a project, paste URLs or DOIs, or add a bibliography export. Downloader will prepare a structured local corpus for review.</p>
    </div>

    <form class="download-form" onsubmit={handleDownload}>
      <ProjectSelector bind:value={projectName} disabled={isDownloading} />

      <div class="field-group">
        <label for="url-input" class="input-label">
          Sources
        </label>
        <p class="field-help">Paste URLs or DOIs one per line. Bibliography files can be added below.</p>
        <textarea
          id="url-input"
          class="url-input"
          bind:value={inputText}
          placeholder="https://arxiv.org/abs/2301.00001&#10;10.1000/xyz123"
          rows={6}
          disabled={isDownloading}
          aria-label="URLs or DOIs to download"
        ></textarea>

        <div class="example-block" aria-label="Input examples">
          <p class="example-label">Examples</p>
          <div class="example-list">
            <code>https://arxiv.org/abs/2301.00001</code>
            <code>10.1000/xyz123</code>
            <p>Add a <code>.bib</code> or <code>.ris</code> export from Zotero, Mendeley, or EndNote.</p>
          </div>
        </div>
      </div>

      <div class="bib-row">
        <button
          type="button"
          class="add-bib-btn"
          disabled={isDownloading}
          onclick={handleAddBibFiles}
        >
          Add .bib / .ris file
        </button>
        {#if bibFiles.length > 0}
          <ul class="bib-chips">
            {#each bibFiles as filePath, i}
              <li class="bib-chip">
                <span class="bib-chip-name">{filePath.split(/[\\/]/).pop() ?? filePath}</span>
                <button
                  type="button"
                  class="bib-chip-remove"
                  aria-label="Remove {filePath}"
                  onclick={() => removeBibFile(i)}
                >×</button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
      {#if bibPickerError}
        <p class="bib-picker-error" role="alert">{bibPickerError}</p>
      {/if}

      <AuthPanel />

      <div class="button-row">
        <button
          type="submit"
          class="download-btn"
          disabled={isInputEmpty || isDownloading}
          aria-busy={isDownloading}
        >
          {isDownloading ? 'Downloading…' : 'Download'}
        </button>

        {#if isDownloading}
          <button
            type="button"
            class="cancel-btn"
            onclick={handleCancel}
            disabled={cancelRequested}
          >
            Cancel
          </button>
        {/if}
      </div>
    </form>
  </section>

  {#if isDownloading}
    <section class="panel panel--run">
      <div class="panel-inline-header">
        <div>
          <p class="panel-kicker">Run state</p>
          <h3>Track the intake as it resolves, downloads, and finishes.</h3>
        </div>
      </div>

      {#if progressPayload}
        <ProgressDisplay payload={progressPayload} />
      {:else}
        <p class="starting-hint" aria-live="polite">Resolving sources…</p>
      {/if}
    </section>
  {/if}

  {#if status === 'done' && summary}
    <section class="panel panel--review">
      <CompletionSummary {summary} {cancelled} onReset={handleReset} />
    </section>
  {/if}

  {#if status === 'error'}
    <section class="panel panel--review">
      <StatusDisplay {status} {message} />
    </section>
  {/if}
</div>

<style>
  .download-form-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .download-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .panel {
    padding: 1.2rem;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.5), transparent), var(--bg-surface);
    border: 1px solid rgba(180, 175, 157, 0.72);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-panel);
  }

  .panel--run {
    background: linear-gradient(180deg, rgba(224, 234, 223, 0.45), transparent), var(--bg-surface);
  }

  .panel-header,
  .panel-inline-header {
    display: grid;
    gap: 0.45rem;
    margin-bottom: 1rem;
  }

  .panel-header {
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    align-items: end;
  }

  .panel-kicker {
    margin: 0 0 0.35rem;
    color: var(--accent-primary);
    font-size: 0.74rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h2,
  h3 {
    margin: 0;
    color: var(--fg-strong);
    font-family: var(--font-display);
    line-height: 1.2;
  }

  h2 {
    font-size: 1.5rem;
    font-weight: 600;
  }

  h3 {
    font-size: 1.15rem;
    font-weight: 600;
  }

  .panel-copy {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.95rem;
    line-height: 1.55;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--fg-strong);
  }

  .field-help {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.86rem;
    line-height: 1.45;
  }

  .url-input {
    width: 100%;
    padding: 0.85rem 0.95rem;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-field);
    background: rgba(255, 255, 255, 0.62);
    color: var(--fg-strong);
    font-family: var(--font-mono);
    font-size: 0.86rem;
    line-height: 1.55;
    resize: vertical;
    box-sizing: border-box;
    transition: border-color 0.2s ease, box-shadow 0.2s ease, background 0.2s ease;
  }

  .url-input:focus {
    border-color: rgba(53, 91, 70, 0.55);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.6), 0 0 0 4px rgba(53, 91, 70, 0.09);
    background: rgba(255, 255, 255, 0.82);
  }

  .url-input:disabled {
    background: rgba(240, 236, 226, 0.82);
    color: var(--fg-muted);
  }

  .example-block {
    padding: 0.8rem 0.9rem;
    border: 1px solid rgba(180, 175, 157, 0.7);
    border-radius: 14px;
    background: rgba(246, 240, 228, 0.78);
  }

  .example-label {
    margin: 0 0 0.45rem;
    color: var(--fg-strong);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .example-list {
    display: grid;
    gap: 0.4rem;
    color: var(--fg-muted);
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .example-list code {
    display: inline-flex;
    width: fit-content;
    padding: 0.15rem 0.4rem;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.7);
    color: var(--fg-strong);
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }

  .example-list p {
    margin: 0;
  }

  .bib-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.65rem;
    align-items: flex-start;
  }

  .add-bib-btn {
    background: transparent;
    color: var(--accent-primary);
    border: 1px solid rgba(53, 91, 70, 0.35);
    border-radius: 999px;
    padding: 0.5rem 0.9rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.18s ease, border-color 0.18s ease;
  }

  .add-bib-btn:hover:not(:disabled) {
    background: rgba(224, 234, 223, 0.78);
    border-color: rgba(53, 91, 70, 0.5);
  }

  .add-bib-btn:disabled {
    color: var(--fg-muted);
    border-color: rgba(180, 175, 157, 0.7);
    cursor: not-allowed;
  }

  .bib-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .bib-chip {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    background: var(--accent-soft);
    border: 1px solid rgba(53, 91, 70, 0.16);
    border-radius: 999px;
    padding: 0.28rem 0.55rem;
    font-size: 0.8rem;
    color: var(--accent-primary);
  }

  .bib-chip-name {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bib-chip-remove {
    background: none;
    border: none;
    color: var(--accent-primary);
    cursor: pointer;
    padding: 0;
    font-size: 1rem;
    line-height: 1;
    opacity: 0.7;
  }

  .bib-chip-remove:hover {
    opacity: 1;
  }

  .bib-picker-error {
    color: var(--state-error);
    font-size: 0.82rem;
    margin: 0;
  }

  .button-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .download-btn {
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: 999px;
    padding: 0.72rem 1.35rem;
    font-size: 0.92rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.18s ease, transform 0.18s ease;
    box-shadow: 0 10px 24px rgba(53, 91, 70, 0.2);
  }

  .download-btn:hover:not(:disabled) {
    background: var(--accent-primary-hover);
    transform: translateY(-1px);
  }

  .download-btn:disabled {
    background: #9ba79e;
    cursor: not-allowed;
    box-shadow: none;
  }

  .cancel-btn {
    background: transparent;
    color: var(--state-error);
    border: 1px solid rgba(143, 68, 54, 0.35);
    border-radius: 999px;
    padding: 0.7rem 1rem;
    font-size: 0.88rem;
    cursor: pointer;
  }

  .cancel-btn:hover:not(:disabled) {
    background: rgba(246, 228, 223, 0.75);
  }

  .cancel-btn:disabled {
    color: var(--fg-muted);
    border-color: rgba(180, 175, 157, 0.8);
    cursor: not-allowed;
  }

  .starting-hint {
    margin: 0;
    padding: 0.85rem 1rem;
    border-radius: 14px;
    background: var(--accent-soft);
    color: var(--accent-primary);
    font-size: 0.9rem;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  @media (max-width: 760px) {
    .panel-header {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .panel {
      padding: 1rem;
    }

    h2 {
      font-size: 1.3rem;
    }

    .download-btn,
    .cancel-btn {
      width: 100%;
      justify-content: center;
    }
  }
</style>
