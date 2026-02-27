<script lang="ts">
  let {
    status = 'idle',
    message = '',
  }: { status?: 'idle' | 'downloading' | 'done' | 'error'; message?: string } = $props();
</script>

<div class="status status--{status}" role="status" aria-live="polite">
  {#if status === 'idle'}
    <span class="status__icon">⬇️</span>
    <span class="status__text">Ready to download</span>
  {:else if status === 'downloading'}
    <span class="status__icon status__icon--spin">⏳</span>
    <span class="status__text">Downloading…</span>
  {:else if status === 'done'}
    <span class="status__icon">✅</span>
    <span class="status__text">{message}</span>
  {:else if status === 'error'}
    <span class="status__icon">❌</span>
    <pre class="status__error">{message}</pre>
  {/if}
</div>

<style>
  .status {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    margin-top: 1rem;
    font-size: 0.9rem;
  }

  .status--idle {
    background: #f0f0f0;
    color: #555;
  }

  .status--downloading {
    background: #e8f4fd;
    color: #1a6b9a;
  }

  .status--done {
    background: #e8f8e8;
    color: #2a6b2a;
  }

  .status--error {
    background: #fef0f0;
    color: #8b1a1a;
    flex-direction: column;
  }

  .status__error {
    margin: 0;
    white-space: pre-wrap;
    font-family: inherit;
    font-size: 0.85rem;
  }

  .status__icon--spin {
    display: inline-block;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
