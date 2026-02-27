<script lang="ts">
  import { formatBytes, urlDomain } from './utils.js';

  interface InProgressItem {
    url: string;
    bytes_downloaded: number;
    content_length: number | null;
  }

  export interface ProgressPayload {
    completed: number;
    failed: number;
    total: number;
    in_progress: InProgressItem[];
  }

  let { payload }: { payload: ProgressPayload } = $props();
</script>

<div class="progress-display" role="region" aria-label="Download progress">
  <!-- Aggregate progress bar -->
  <div class="progress-header">
    <span class="progress-counts">
      {payload.completed} / {payload.total}
      {#if payload.failed > 0}
        <span class="failed-badge">{payload.failed} failed</span>
      {/if}
    </span>
    {#if payload.completed + payload.failed < payload.total}
      <span class="spinner" aria-hidden="true">⟳</span>
    {/if}
  </div>

  <progress
    class="progress-bar"
    value={payload.completed + payload.failed}
    max={payload.total}
    aria-label="Overall progress: {payload.completed + payload.failed} of {payload.total}"
  ></progress>

  <!-- Per-item list -->
  {#if payload.in_progress.length > 0}
    <ul class="in-progress-list" aria-label="Active downloads">
      {#each payload.in_progress as item (item.url)}
        <li class="in-progress-item">
          <span class="item-domain">{urlDomain(item.url)}</span>
          <span class="item-bytes">
            {#if item.content_length && item.content_length > 0}
              {formatBytes(item.bytes_downloaded)} / {formatBytes(item.content_length)}
            {:else}
              {formatBytes(item.bytes_downloaded)}
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .progress-display {
    margin: 1rem 0;
  }

  .progress-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.4rem;
    font-size: 0.9rem;
  }

  .progress-counts {
    font-weight: 600;
    color: #1a1a2e;
  }

  .failed-badge {
    color: #c0392b;
    font-size: 0.8rem;
    font-weight: 500;
  }

  .spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
    font-size: 1rem;
    color: #396cd8;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .progress-bar {
    width: 100%;
    height: 8px;
    border-radius: 4px;
    appearance: none;
    background: #e0e0e0;
  }

  .progress-bar::-webkit-progress-bar {
    background: #e0e0e0;
    border-radius: 4px;
  }

  .progress-bar::-webkit-progress-value {
    background: #396cd8;
    border-radius: 4px;
  }

  .in-progress-list {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
    font-size: 0.82rem;
    color: #555;
  }

  .in-progress-item {
    display: flex;
    justify-content: space-between;
    padding: 0.2rem 0;
    border-bottom: 1px solid #f0f0f0;
  }

  .item-domain {
    font-family: monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }

  .item-bytes {
    color: #888;
    flex-shrink: 0;
  }
</style>
