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

<label for="project-input" class="input-label">Project (optional)</label>
<input
  id="project-input"
  class="project-input"
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

  .project-input:disabled {
    background: #f5f5f5;
    color: #888;
  }
</style>
