<script lang="ts">
  let {
    status = 'idle',
    message = '',
  }: { status?: 'idle' | 'downloading' | 'done' | 'error'; message?: string } = $props();
</script>

<div class="status status--{status}" role="status" aria-live="polite">
  {#if status === 'idle'}
    <span class="status__marker" aria-hidden="true"></span>
    <span class="status__text">Ready to download</span>
  {:else if status === 'downloading'}
    <span class="status__marker status__marker--pulse" aria-hidden="true"></span>
    <span class="status__text">Downloading…</span>
  {:else if status === 'done'}
    <span class="status__marker status__marker--success" aria-hidden="true"></span>
    <span class="status__text">{message}</span>
  {:else if status === 'error'}
    <span class="status__label">Needs attention</span>
    <pre class="status__error">{message}</pre>
  {/if}
</div>

<style>
  .status {
    display: flex;
    align-items: flex-start;
    gap: 0.7rem;
    padding: 1rem 1.05rem;
    border-radius: 16px;
    font-size: 0.9rem;
    border: 1px solid rgba(180, 175, 157, 0.72);
  }

  .status--idle {
    background: rgba(246, 240, 228, 0.78);
    color: var(--fg-muted);
  }

  .status--downloading {
    background: rgba(224, 234, 223, 0.62);
    color: var(--accent-primary);
  }

  .status--done {
    background: rgba(224, 234, 223, 0.62);
    color: var(--state-success);
  }

  .status--error {
    background: rgba(246, 228, 223, 0.72);
    color: var(--state-error);
    flex-direction: column;
  }

  .status__label {
    display: inline-flex;
    align-items: center;
    width: fit-content;
    padding: 0.28rem 0.55rem;
    border-radius: 999px;
    background: rgba(143, 68, 54, 0.1);
    color: var(--state-error);
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .status__marker {
    width: 0.7rem;
    height: 0.7rem;
    margin-top: 0.28rem;
    border-radius: 50%;
    background: var(--fg-muted);
    flex-shrink: 0;
  }

  .status__marker--pulse {
    background: var(--accent-primary);
    animation: pulse 1.1s ease-in-out infinite;
  }

  .status__marker--success {
    background: var(--state-success);
  }

  .status__text {
    line-height: 1.45;
  }

  .status__error {
    margin: 0;
    white-space: pre-wrap;
    color: var(--fg-strong);
    font-family: var(--font-body);
    font-size: 0.85rem;
    line-height: 1.55;
  }

  @keyframes pulse {
    0%, 100% { transform: scale(0.86); opacity: 0.5; }
    50% { transform: scale(1); opacity: 1; }
  }
</style>
