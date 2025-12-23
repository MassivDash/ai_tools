<script lang="ts">
  import { createEventDispatcher } from 'svelte'

  export let id: string = ''
  export let label: string = ''
  export let value: string = ''
  export let options: Array<{ value: string; label: string }> = []
  export let disabled: boolean = false
  export let required: boolean = false

  const dispatch = createEventDispatcher()

  function handleChange(event: Event) {
    const target = event.target as HTMLSelectElement
    value = target.value
    dispatch('change', { value: target.value })
  }
</script>

<div class="select-wrapper">
  {#if label}
    <label for={id} class="select-label">{label}</label>
  {/if}
  <select
    {id}
    {value}
    {disabled}
    {required}
    class="select"
    on:change={handleChange}
    {...$$restProps}
  >
    {#each options as option}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</div>

<style>
  .select-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .select-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: inherit;
  }

  .select {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    font-size: 0.9rem;
    font-family: inherit;
    background-color: var(--bg-primary, white);
    color: var(--text-primary, #333);
    cursor: pointer;
    transition:
      border-color 0.2s,
      box-shadow 0.2s,
      background-color 0.3s ease,
      color 0.3s ease;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%23333' d='M6 9L1 4h10z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    padding-right: 2.5rem;
  }

  .select:hover:not(:disabled) {
    border-color: var(--border-color-hover, #999);
  }

  .select:focus {
    outline: none;
    border-color: var(--accent-color, #2196f3);
    box-shadow: 0 0 0 3px rgba(33, 150, 243, 0.1);
  }

  .select:disabled {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
    opacity: 0.6;
  }

  /* Dark theme - update arrow icon color */
  :global(.dark) .select {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%23fff' d='M6 9L1 4h10z'/%3E%3C/svg%3E");
  }
</style>
