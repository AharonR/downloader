<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let {
    value = $bindable(''),
    disabled = false,
  }: { value: string; disabled?: boolean } = $props();

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
<label for="project-input" class="input-label">Project (optional)</label>
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

<style>
  .input-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: #333;
  }

  .project-input {
    width: 100%;
    padding: 0.6rem 0.8rem;
    border: 1px solid #ccc;
    border-radius: 8px;
    font-size: 0.9rem;
    box-sizing: border-box;
    transition: border-color 0.2s;
  }

  .project-input:focus {
    outline: none;
    border-color: #396cd8;
  }

  .project-input.project-active {
    border-color: #396cd8;
    background: #f0f7ff;
  }

  .project-input.project-active:focus {
    border-color: #396cd8;
    background: #f0f7ff;
  }

  .project-input:disabled {
    background: #f5f5f5;
    color: #888;
  }

  .project-input.project-active:disabled {
    background: #f5f5f5;
  }
</style>
