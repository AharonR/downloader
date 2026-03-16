<script lang="ts">
  import DownloadForm from '$lib/DownloadForm.svelte';
</script>

<svelte:head>
  <title>Downloader</title>
</svelte:head>

<main class="app-shell">
  <article class="workbench">
    <header class="app-header">
      <p class="app-kicker">Local-first evidence intake</p>
      <h1>Downloader</h1>
      <p class="app-subtitle">Turn source lists into organized research corpora.</p>
      <ul class="signal-list" aria-label="Downloader product qualities">
        <li>Project-based</li>
        <li>Structured outputs</li>
        <li>Trust-first workflow</li>
      </ul>
    </header>

    <DownloadForm />
  </article>
</main>

<style>
  :global(:root) {
    --bg-app: #f4efe3;
    --bg-app-muted: #ece6d9;
    --bg-surface: #fbf8f1;
    --bg-panel: rgba(251, 248, 241, 0.84);
    --fg-strong: #232822;
    --fg-default: #343b34;
    --fg-muted: #667164;
    --accent-primary: #355b46;
    --accent-primary-hover: #274335;
    --accent-soft: #e0eadf;
    --state-success: #3f6f51;
    --state-warning: #9b7332;
    --state-error: #8f4436;
    --border-subtle: #d2ccbf;
    --shadow-soft: 0 18px 50px rgba(59, 50, 36, 0.09);
    --shadow-panel: 0 10px 26px rgba(63, 54, 39, 0.08);
    --radius-card: 24px;
    --radius-panel: 18px;
    --radius-field: 14px;
    --font-display: "Source Serif 4", "Iowan Old Style", "Palatino Linotype", Georgia, serif;
    --font-body: "IBM Plex Sans", "Source Sans 3", "Instrument Sans", system-ui, sans-serif;
    --font-mono: "IBM Plex Mono", "JetBrains Mono", "SFMono-Regular", monospace;
  }

  :global(html, body) {
    margin: 0;
    min-height: 100%;
  }

  :global(body) {
    font-family: var(--font-body);
    color: var(--fg-default);
    background:
      radial-gradient(circle at top, rgba(80, 107, 86, 0.13), transparent 32%),
      linear-gradient(180deg, var(--bg-app) 0%, var(--bg-app-muted) 100%);
  }

  :global(button),
  :global(input),
  :global(textarea) {
    font: inherit;
  }

  :global(button:focus-visible),
  :global(input:focus-visible),
  :global(textarea:focus-visible) {
    outline: 2px solid rgba(53, 91, 70, 0.35);
    outline-offset: 2px;
  }

  .app-shell {
    min-height: 100vh;
    padding: 2.5rem 1rem;
    box-sizing: border-box;
  }

  .workbench {
    max-width: 860px;
    margin: 0 auto;
    padding: 2rem;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.55), rgba(255, 255, 255, 0)),
      var(--bg-panel);
    border: 1px solid rgba(180, 175, 157, 0.8);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-soft);
    backdrop-filter: blur(10px);
  }

  .app-header {
    margin-bottom: 1.8rem;
    padding-bottom: 1.4rem;
    border-bottom: 1px solid rgba(180, 175, 157, 0.55);
  }

  .app-kicker {
    margin: 0 0 0.65rem;
    color: var(--accent-primary);
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h1 {
    margin: 0 0 0.35rem;
    color: var(--fg-strong);
    font-family: var(--font-display);
    font-size: clamp(2.2rem, 4vw, 3.1rem);
    font-weight: 600;
    line-height: 1.04;
    letter-spacing: -0.02em;
  }

  .app-subtitle {
    max-width: 34rem;
    margin: 0 0 1rem;
    color: var(--fg-muted);
    font-size: 1rem;
    line-height: 1.55;
  }

  .signal-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.55rem;
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .signal-list li {
    padding: 0.42rem 0.72rem;
    border-radius: 999px;
    border: 1px solid rgba(53, 91, 70, 0.18);
    background: rgba(224, 234, 223, 0.7);
    color: var(--accent-primary);
    font-size: 0.82rem;
    font-weight: 600;
  }

  @media (max-width: 720px) {
    .app-shell {
      padding: 1.25rem 0.75rem;
    }

    .workbench {
      padding: 1.15rem;
      border-radius: 20px;
    }

    .signal-list li {
      font-size: 0.78rem;
    }
  }
</style>
