<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let {
    value = $bindable(''),
    disabled = false,
  }: { value?: string; disabled?: boolean } = $props();

  let projects = $state<string[]>([]);

  onMount(async () => {
    try {
      projects = await invoke<string[]>('list_projects');
    } catch {
      // Ignore — empty list is fine; app may not be running in Tauri context
    }
  });
</script>

<!--
  Keyboard navigation decision: this component uses a native <datalist> element
  rather than a custom dropdown. Native <datalist> provides arrow-key + Enter
  navigation out of the box in Chrome and Safari without any additional event
  handlers. Firefox shows a custom UI as well. This avoids re-implementing
  accessibility (focus management, ARIA live regions, keyboard trap prevention)
  that browsers already handle correctly for the <input list="..."> pattern.
  If richer filtering or multi-select is ever needed, replace with a dedicated
  accessible combobox component at that point.
-->
<div class="project-field">
  <label for="project-input" class="input-label">Project (optional)</label>
  <p class="input-help">Group this run into a reusable folder and keep related outputs together.</p>
  <input
    id="project-input"
    class="project-input"
    class:project-active={value.trim().length > 0}
    list="project-suggestions"
    bind:value
    placeholder="e.g. Climate Research"
    {disabled}
  />
  <datalist id="project-suggestions">
    {#each projects as name}
      <option value={name}></option>
    {/each}
  </datalist>
</div>

<style>
  .project-field {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .input-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--fg-strong);
  }

  .input-help {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .project-input {
    width: 100%;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-field);
    background: rgba(255, 255, 255, 0.62);
    color: var(--fg-strong);
    font-size: 0.9rem;
    box-sizing: border-box;
    transition: border-color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease;
  }

  .project-input:focus {
    border-color: rgba(53, 91, 70, 0.55);
    box-shadow: 0 0 0 4px rgba(53, 91, 70, 0.09);
    background: rgba(255, 255, 255, 0.82);
  }

  .project-input.project-active {
    border-color: rgba(53, 91, 70, 0.4);
    background: rgba(224, 234, 223, 0.52);
  }

  .project-input.project-active:focus {
    border-color: rgba(53, 91, 70, 0.55);
    background: rgba(224, 234, 223, 0.72);
  }

  .project-input:disabled {
    background: rgba(240, 236, 226, 0.82);
    color: var(--fg-muted);
  }

  .project-input.project-active:disabled {
    background: rgba(224, 234, 223, 0.4);
  }
</style>
