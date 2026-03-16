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

  const processed = $derived(payload.completed + payload.failed);
  const stageLabel = $derived(
    processed === 0 && payload.in_progress.length === 0
      ? 'Resolving sources'
      : processed < payload.total
        ? 'Downloading files'
        : 'Finishing run',
  );
  const stageCopy = $derived(
    payload.failed > 0
      ? `${payload.failed} item${payload.failed !== 1 ? 's' : ''} need attention while the rest of the run continues.`
      : 'Building a structured local result set from the sources you provided.',
  );
</script>

<div class="progress-display" role="region" aria-label="Download progress">
  <div class="progress-header">
    <div class="stage-column">
      <div class="stage-badge">
        {#if processed < payload.total}
          <span class="spinner" aria-hidden="true"></span>
        {/if}
        {stageLabel}
      </div>
      <p class="stage-copy">{stageCopy}</p>
    </div>
    <div class="progress-summary">
      <span class="progress-counts">{processed} / {payload.total}</span>
      {#if payload.failed > 0}
        <span class="failed-badge">{payload.failed} failed</span>
      {/if}
    </div>
  </div>

  <progress
    class="progress-bar"
    value={processed}
    max={payload.total}
    aria-label="Overall progress: {processed} of {payload.total}"
  ></progress>

  {#if payload.in_progress.length > 0}
    <ul class="in-progress-list" aria-label="Active downloads">
      {#each payload.in_progress as item (item.url)}
        <li class="in-progress-item">
          <div class="item-main">
            <span class="item-domain">{urlDomain(item.url)}</span>
            <span class="item-source">{item.url}</span>
          </div>
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
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .stage-column {
    display: grid;
    gap: 0.4rem;
  }

  .stage-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    width: fit-content;
    padding: 0.35rem 0.7rem;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-primary);
    font-size: 0.82rem;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .stage-copy {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.88rem;
    line-height: 1.45;
  }

  .progress-summary {
    display: grid;
    justify-items: end;
    gap: 0.25rem;
    text-align: right;
  }

  .progress-counts {
    color: var(--fg-strong);
    font-family: var(--font-mono);
    font-size: 0.88rem;
    font-weight: 600;
  }

  .failed-badge {
    color: var(--state-error);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .spinner {
    width: 0.52rem;
    height: 0.52rem;
    display: inline-block;
    border-radius: 50%;
    background: var(--accent-primary);
    animation: pulse 1.15s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { transform: scale(0.85); opacity: 0.45; }
    50% { transform: scale(1); opacity: 1; }
  }

  .progress-bar {
    width: 100%;
    height: 10px;
    border-radius: 999px;
    appearance: none;
    background: rgba(210, 204, 191, 0.6);
    overflow: hidden;
  }

  .progress-bar::-webkit-progress-bar {
    background: rgba(210, 204, 191, 0.6);
    border-radius: 999px;
  }

  .progress-bar::-webkit-progress-value {
    background: linear-gradient(90deg, var(--accent-primary), #547a63);
    border-radius: 999px;
  }

  .progress-bar::-moz-progress-bar {
    background: linear-gradient(90deg, var(--accent-primary), #547a63);
    border-radius: 999px;
  }

  .in-progress-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.5rem;
  }

  .in-progress-item {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    padding: 0.8rem 0.9rem;
    border: 1px solid rgba(180, 175, 157, 0.62);
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.58);
  }

  .item-main {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
  }

  .item-domain {
    color: var(--fg-strong);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-source {
    color: var(--fg-muted);
    font-size: 0.78rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-bytes {
    color: var(--fg-muted);
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }

  @media (max-width: 680px) {
    .progress-header,
    .in-progress-item {
      flex-direction: column;
    }

    .progress-summary {
      justify-items: start;
      text-align: left;
    }
  }
</style>
