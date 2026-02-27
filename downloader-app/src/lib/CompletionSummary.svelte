<script lang="ts">
  export interface DownloadSummary {
    completed: number;
    failed: number;
    output_dir: string;
  }

  let {
    summary,
    onReset,
    cancelled = false,
  }: { summary: DownloadSummary; onReset: () => void; cancelled?: boolean } = $props();
</script>

<div class="completion-summary" role="region" aria-label="Download complete">
  {#if cancelled}
    <p class="status-cancelled">
      Cancelled — {summary.completed} completed, {summary.failed} failed
    </p>
    {#if summary.failed > 0}
      <p class="error-hint">
        What: Some downloads did not complete before cancellation.<br />
        Why: Downloads in flight were interrupted; others may have failed due to network or resolution errors.<br />
        Fix: Re-run the download or check network connectivity.
      </p>
    {/if}
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
    {#if summary.failed > 0}
      <p class="error-hint">
        What: Some downloads failed.<br />
        Why: Network errors, paywalled content, or unresolvable DOIs.<br />
        Fix: Check your network connection and verify URLs/DOIs are accessible.
      </p>
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

  .error-hint {
    font-size: 0.85rem;
    color: #c0392b;
    background: #fff5f5;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    border-left: 3px solid #c0392b;
    margin: 0.5rem 0 0.75rem;
    line-height: 1.6;
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
