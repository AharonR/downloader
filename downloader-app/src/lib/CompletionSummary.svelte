<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  export interface FailedItem {
    input: string;
    error: string;
  }

  export interface DownloadWarning {
    code: string;
    path: string;
    error: string;
    impact: string;
    fix: string;
  }

  export interface DownloadSummary {
    completed: number;
    failed: number;
    skipped_duplicates?: number;
    output_dir: string;
    failed_items: FailedItem[];
    warnings?: DownloadWarning[];
  }

  let {
    summary,
    onReset,
    cancelled = false,
  }: { summary: DownloadSummary; onReset: () => void; cancelled?: boolean } = $props();

  let showFailedDetails = $state(false);
  let openFolderError = $state<string | null>(null);
  const EXPAND_ALL_THRESHOLD = 5;
  let expandAll = $state(false);
  const skippedDuplicates = $derived(summary.skipped_duplicates ?? 0);
  const warnings = $derived(summary.warnings ?? []);

  async function handleOpenFolder() {
    openFolderError = null;
    try {
      await invoke('open_folder', { path: summary.output_dir });
    } catch (err) {
      openFolderError = typeof err === 'string' ? err : 'Could not open folder';
    }
  }

  function toggleExpandAll() {
    expandAll = !expandAll;
    showFailedDetails = expandAll;
  }
</script>

<div
  class="completion-summary"
  class:summary-success={!cancelled && summary.failed === 0}
  class:summary-partial={!cancelled && summary.failed > 0 && summary.completed > 0}
  class:summary-all-failed={!cancelled && summary.failed > 0 && summary.completed === 0}
  class:summary-cancelled={cancelled}
  role="region"
  aria-label="Download complete"
>
  <div class="summary-heading">
    <div>
      <p class="summary-kicker">Results</p>
      {#if cancelled}
        <h3 class="status-cancelled">Run stopped</h3>
        <p class="status-copy">You still have the files that completed before the run was cancelled.</p>
      {:else if summary.completed === 0 && summary.failed === 0 && skippedDuplicates > 0}
        <h3 class="status-success">Run complete</h3>
        <p class="status-copy">No new files were needed because matching items were already downloaded.</p>
      {:else if summary.failed === 0}
        <h3 class="status-success">Run complete</h3>
        <p class="status-copy">The corpus is ready for review and handoff.</p>
      {:else}
        <h3 class="status-partial">Run complete with items to review</h3>
        <p class="status-copy">Most of the work completed. Review the items that need attention before your next step.</p>
      {/if}
    </div>
  </div>

  <div class="summary-metrics" aria-label="Run summary metrics">
    <div class="metric-card">
      <span class="metric-label">Completed</span>
      <strong class="metric-value">{summary.completed}</strong>
    </div>
    <div class="metric-card">
      <span class="metric-label">Needs attention</span>
      <strong class="metric-value">{summary.failed}</strong>
    </div>
    <div class="metric-card">
      <span class="metric-label">Already downloaded</span>
      <strong class="metric-value">{skippedDuplicates}</strong>
    </div>
    <div class="metric-card">
      <span class="metric-label">Status</span>
      <strong class="metric-value metric-value--text">
        {#if cancelled}
          Cancelled
        {:else if summary.completed === 0 && summary.failed === 0 && skippedDuplicates > 0}
          No-op
        {:else if summary.failed === 0}
          Ready
        {:else}
          Mixed
        {/if}
      </strong>
    </div>
  </div>

  {#if summary.output_dir && (summary.completed > 0 || skippedDuplicates > 0)}
    <div class="output-block">
      <div class="output-copy">
        <p class="output-label">Project output</p>
        <code class="output-dir">{summary.output_dir}</code>
      </div>
      <button class="open-folder-btn" onclick={handleOpenFolder} type="button">
        Open output folder
      </button>
    </div>
  {/if}

  {#if summary.failed_items.length > 0}
    <div class="failed-controls">
      <button
        class="toggle-details"
        type="button"
        aria-expanded={showFailedDetails}
        onclick={() => { showFailedDetails = !showFailedDetails; }}
      >
        {showFailedDetails ? 'Hide' : 'Show'} failed items ({summary.failed_items.length})
      </button>
      {#if summary.failed_items.length > EXPAND_ALL_THRESHOLD}
        <button
          class="toggle-expand-all"
          type="button"
          onclick={toggleExpandAll}
        >
          {expandAll ? 'Collapse all' : 'Expand all'}
        </button>
      {/if}
    </div>

    {#if showFailedDetails}
      <ul class="failed-list">
        {#each summary.failed_items as item}
          <li class="failed-item">
            <span class="failed-input">{item.input}</span>
            <span class="failed-error">{item.error}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  {#if warnings.length > 0}
    <div class="warning-block" role="status" aria-label="Post-run warnings">
      <p class="warning-heading">Warnings</p>
      <ul class="warning-list">
        {#each warnings as warning}
          <li class="warning-item">
            <p class="warning-line"><code>{warning.code}</code>: {warning.error}</p>
            <p class="warning-meta">Path: <code>{warning.path}</code></p>
            <p class="warning-meta">Impact: {warning.impact}</p>
            <p class="warning-meta">Fix: {warning.fix}</p>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="action-row">
    <button class="reset-btn" onclick={onReset} type="button">
      Download more
    </button>
  </div>

  {#if openFolderError}
    <p class="open-folder-error" role="alert">{openFolderError}</p>
  {/if}
</div>

<style>
  .completion-summary {
    display: flex;
    flex-direction: column;
    gap: 0.95rem;
  }

  .summary-heading {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
  }

  .summary-kicker {
    margin: 0 0 0.3rem;
    color: var(--accent-primary);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.4rem;
    font-weight: 600;
    line-height: 1.15;
  }

  .status-success {
    color: var(--state-success);
  }

  .status-partial {
    color: var(--state-warning);
  }

  .status-cancelled {
    color: var(--fg-strong);
  }

  .status-copy {
    margin: 0.35rem 0 0;
    color: var(--fg-muted);
    font-size: 0.92rem;
    line-height: 1.5;
  }

  .summary-metrics {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.7rem;
  }

  .metric-card {
    padding: 0.9rem 1rem;
    border: 1px solid rgba(180, 175, 157, 0.72);
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.6);
  }

  .metric-label {
    display: block;
    margin-bottom: 0.35rem;
    color: var(--fg-muted);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .metric-value {
    color: var(--fg-strong);
    font-family: var(--font-mono);
    font-size: 1.3rem;
    font-weight: 600;
  }

  .metric-value--text {
    font-family: var(--font-body);
    font-size: 1rem;
  }

  .output-block {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: center;
    padding: 0.95rem 1rem;
    border: 1px solid rgba(53, 91, 70, 0.18);
    border-radius: 16px;
    background: rgba(224, 234, 223, 0.46);
  }

  .output-copy {
    display: grid;
    gap: 0.35rem;
    min-width: 0;
  }

  .output-label {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .warning-block {
    padding: 0.85rem 0.95rem;
    border: 1px solid rgba(158, 118, 25, 0.38);
    border-radius: 14px;
    background: rgba(249, 238, 211, 0.7);
  }

  .warning-heading {
    margin: 0 0 0.5rem;
    color: var(--fg-strong);
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .warning-list {
    margin: 0;
    padding-left: 1rem;
    display: grid;
    gap: 0.5rem;
  }

  .warning-item {
    color: var(--fg-strong);
    font-size: 0.86rem;
    line-height: 1.45;
  }

  .warning-line {
    margin: 0;
    font-weight: 600;
  }

  .warning-meta {
    margin: 0.12rem 0 0;
    color: var(--fg-muted);
  }

  .output-dir {
    display: inline-block;
    max-width: 100%;
    overflow-wrap: anywhere;
    padding: 0.12rem 0;
    color: var(--fg-strong);
    font-family: var(--font-mono);
    font-size: 0.82rem;
    background: transparent;
  }

  .failed-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .toggle-details {
    background: none;
    border: 1px solid rgba(143, 68, 54, 0.35);
    color: var(--state-error);
    border-radius: 999px;
    padding: 0.45rem 0.75rem;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
  }

  .toggle-details:hover {
    background: rgba(246, 228, 223, 0.75);
  }

  .toggle-expand-all {
    background: none;
    border: 1px solid rgba(180, 175, 157, 0.85);
    color: var(--fg-muted);
    border-radius: 999px;
    padding: 0.45rem 0.75rem;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .toggle-expand-all:hover {
    background: rgba(246, 240, 228, 0.78);
  }

  .failed-list {
    list-style: none;
    padding: 0;
    margin: 0;
    border: 1px solid rgba(143, 68, 54, 0.25);
    background: rgba(246, 228, 223, 0.68);
    border-radius: 16px;
  }

  .failed-item {
    padding: 0.7rem 0.85rem;
    font-size: 0.83rem;
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
  }

  .failed-item + .failed-item {
    border-top: 1px solid rgba(143, 68, 54, 0.12);
  }

  .failed-input {
    color: var(--fg-strong);
    font-family: var(--font-mono);
    word-break: break-all;
  }

  .failed-error {
    color: var(--state-error);
    line-height: 1.4;
  }

  .action-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .open-folder-btn {
    background: rgba(255, 255, 255, 0.7);
    color: var(--accent-primary);
    border: 1px solid rgba(53, 91, 70, 0.32);
    border-radius: 999px;
    padding: 0.58rem 1rem;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .open-folder-btn:hover {
    background: rgba(255, 255, 255, 0.92);
  }

  .reset-btn {
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: 999px;
    padding: 0.62rem 1.12rem;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
  }

  .reset-btn:hover {
    background: var(--accent-primary-hover);
  }

  .open-folder-error {
    color: var(--state-error);
    font-size: 0.82rem;
    margin: 0;
  }

  @media (max-width: 680px) {
    .summary-metrics {
      grid-template-columns: 1fr;
    }

    .output-block {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
