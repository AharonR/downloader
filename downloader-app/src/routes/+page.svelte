<script lang="ts">
  import DownloadForm from '$lib/DownloadForm.svelte';
</script>

<svelte:head>
  <title>Downloader</title>
</svelte:head>

<main>
  <header class="app-header">
    <h1>Downloader</h1>
    <p class="app-subtitle">Paste URLs or DOIs and download papers to your local library</p>
  </header>

  <DownloadForm />
</main>

<style>
  main {
    min-height: 100vh;
    padding: 2rem 1rem;
    background: #fafafa;
  }

  .app-header {
    text-align: center;
    margin-bottom: 2rem;
  }

  h1 {
    font-size: 2rem;
    font-weight: 700;
    margin: 0 0 0.25rem;
    color: #1a1a2e;
  }

  .app-subtitle {
    color: #555;
    margin: 0;
    font-size: 0.95rem;
  }
</style>
