<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onDestroy } from 'svelte';
  import StatusDisplay from './StatusDisplay.svelte';
  import ProgressDisplay from './ProgressDisplay.svelte';
  import CompletionSummary from './CompletionSummary.svelte';
  import ProjectSelector from './ProjectSelector.svelte';
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

  // Unlisten function for the progress event listener (not reactive — not rendered).
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
      // Set up progress event listener before invoking so no events are missed.
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
  <form class="download-form" onsubmit={handleDownload}>
    <ProjectSelector bind:value={projectName} disabled={isDownloading} />

    <label for="url-input" class="input-label">
      URLs or DOIs (one per line)
    </label>
    <textarea
      id="url-input"
      class="url-input"
      bind:value={inputText}
      placeholder="https://arxiv.org/abs/2301.00001&#10;10.1000/xyz123&#10;&#10;Or use &quot;Add .bib / .ris file&quot; to import from Zotero, Mendeley, or EndNote"
      rows={5}
      disabled={isDownloading}
      aria-label="URLs or DOIs to download"
    ></textarea>

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

  <!-- Progress display while downloading -->
  {#if isDownloading && progressPayload}
    <ProgressDisplay payload={progressPayload} />
  {:else if isDownloading && !progressPayload}
    <p class="starting-hint" aria-live="polite">Resolving…</p>
  {/if}

  <!-- Completion summary -->
  {#if status === 'done' && summary}
    <CompletionSummary {summary} {cancelled} onReset={handleReset} />
  {/if}

  <!-- Error display -->
  {#if status === 'error'}
    <StatusDisplay {status} {message} />
  {/if}
</div>

<style>
  .download-form-container {
    max-width: 600px;
    margin: 0 auto;
    padding: 1.5rem;
    background: white;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .download-form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .input-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: #333;
  }

  .url-input {
    width: 100%;
    padding: 0.6rem 0.8rem;
    border: 1px solid #ccc;
    border-radius: 8px;
    font-family: monospace;
    font-size: 0.85rem;
    resize: vertical;
    box-sizing: border-box;
    transition: border-color 0.2s;
  }

  .url-input:focus {
    outline: none;
    border-color: #396cd8;
  }

  .url-input:disabled {
    background: #f5f5f5;
    color: #888;
  }

  .bib-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .add-bib-btn {
    background: transparent;
    color: #396cd8;
    border: 1px solid #396cd8;
    border-radius: 6px;
    padding: 0.35rem 0.85rem;
    font-size: 0.85rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .add-bib-btn:hover:not(:disabled) {
    background: #eef2fc;
  }

  .add-bib-btn:disabled {
    color: #999;
    border-color: #ccc;
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
    background: #e8f5e9;
    border: 1px solid #a5d6a7;
    border-radius: 12px;
    padding: 0.2rem 0.5rem;
    font-size: 0.8rem;
    color: #2e7d32;
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
    color: #2e7d32;
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
    color: #c0392b;
    font-size: 0.82rem;
    margin: 0;
  }

  .button-row {
    display: flex;
    gap: 0.5rem;
  }

  .download-btn {
    background: #396cd8;
    color: white;
    border: none;
    border-radius: 8px;
    padding: 0.55rem 1.4rem;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .download-btn:hover:not(:disabled) {
    background: #2a55c0;
  }

  .download-btn:disabled {
    background: #b0bec5;
    cursor: not-allowed;
  }

  .cancel-btn {
    background: transparent;
    color: #c0392b;
    border: 1px solid #c0392b;
    border-radius: 8px;
    padding: 0.5rem 1rem;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.2s;
  }

  .cancel-btn:hover:not(:disabled) {
    background: #fff0f0;
  }

  .cancel-btn:disabled {
    color: #999;
    border-color: #ccc;
    cursor: not-allowed;
  }

  .starting-hint {
    color: #888;
    font-size: 0.85rem;
    margin: 0.5rem 0 0;
  }
</style>
