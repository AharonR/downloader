<script lang="ts">
  export interface FailedItem {
    input: string;
    error: string;
  }

  export interface DownloadSummary {
    completed: number;
    failed: number;
    output_dir: string;
    failed_items: FailedItem[];
  }

  let {
    summary,
    onReset,
    cancelled = false,
  }: { summary: DownloadSummary; onReset: () => void; cancelled?: boolean } = $props();

  let showFailedDetails = $state(false);
  // Threshold above which an expand/collapse all toggle is shown.
  const EXPAND_ALL_THRESHOLD = 5;
  let expandAll = $state(false);

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
  {#if cancelled}
    <p class="status-cancelled">
      Cancelled — {summary.completed} completed, {summary.failed} failed
    </p>
  {:else if summary.failed === 0}
    <p class="status-success">
      Downloaded {summary.completed} file{summary.completed !== 1 ? 's' : ''} to
      <code class="output-dir">{summary.output_dir}</code>
    </p>
  {:else}
    <p class="status-partial">
      Completed: {summary.completed} downloaded, {summary.failed} failed
      {#if summary.completed > 0}
        (saved to <code class="output-dir">{summary.output_dir}</code>)
      {/if}
    </p>
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

  <button class="reset-btn" onclick={onReset} type="button">
    Download more
  </button>
</div>

<style>
  .completion-summary {
    padding: 1rem;
    border-radius: 8px;
    background: #f0f7ff;
    margin-top: 1rem;
    border-top: 3px solid transparent;
  }

  .summary-success {
    border-top-color: #1a7a4a;
  }

  .summary-partial {
    border-top-color: #c07000;
  }

  .summary-all-failed {
    border-top-color: #c0392b;
  }

  .summary-cancelled {
    border-top-color: #888;
  }

  .status-success {
    color: #1a7a4a;
    font-weight: 600;
    margin: 0 0 0.75rem;
  }

  .status-partial {
    color: #c07000;
    font-weight: 600;
    margin: 0 0 0.5rem;
  }

  .status-cancelled {
    color: #666;
    font-weight: 600;
    margin: 0 0 0.75rem;
  }

  .output-dir {
    background: #e8e8f0;
    padding: 0.1em 0.4em;
    border-radius: 4px;
    font-size: 0.85em;
  }

  .failed-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .toggle-details {
    background: none;
    border: 1px solid #c0392b;
    color: #c0392b;
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .toggle-details:hover {
    background: #fff5f5;
  }

  .toggle-expand-all {
    background: none;
    border: 1px solid #888;
    color: #555;
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .toggle-expand-all:hover {
    background: #f5f5f5;
  }

  .failed-list {
    list-style: none;
    padding: 0;
    margin: 0 0 0.75rem;
    border-left: 3px solid #c0392b;
    background: #fff5f5;
    border-radius: 0 4px 4px 0;
  }

  .failed-item {
    padding: 0.4rem 0.6rem;
    font-size: 0.83rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .failed-item + .failed-item {
    border-top: 1px solid #f5d5d5;
  }

  .failed-input {
    font-family: monospace;
    color: #333;
    word-break: break-all;
  }

  .failed-error {
    color: #c0392b;
    line-height: 1.4;
  }

  .reset-btn {
    background: #396cd8;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.5rem 1.2rem;
    font-size: 0.9rem;
    cursor: pointer;
    margin-top: 0.25rem;
  }

  .reset-btn:hover {
    background: #2a55c0;
  }
</style>
