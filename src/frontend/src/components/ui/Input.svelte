<script lang="ts">
  export let value: string | number = ''
  export let type: string = 'text'
  export let placeholder: string = ''
  export let label: string = ''
  export let id: string = ''
  export let required: boolean = false
  export let disabled: boolean = false
  export let hint: string = ''

  $: inputId = id || `input-${Math.random().toString(36).substr(2, 9)}`
</script>

<div class="input-wrapper">
  {#if label}
    <label for={inputId} class="input-label">
      {label}
      {#if required}
        <span class="required">*</span>
      {/if}
    </label>
  {/if}
  <input
    {id}
    {type}
    {placeholder}
    {required}
    {disabled}
    bind:value
    class="input"
    on:input
    on:change
    {...$$restProps}
  />
  {#if hint}
    <p class="input-hint">{hint}</p>
  {/if}
</div>

<style>
  .input-wrapper {
    margin-bottom: 1rem;
  }

  .input-label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .required {
    color: var(--accent-color, #f44336);
    margin-left: 0.25rem;
    transition: color 0.3s ease;
  }

  .input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    font-size: 1rem;
    font-family: inherit;
    box-sizing: border-box;
    background-color: var(--bg-primary, #fff);
    color: var(--text-primary, #333);
    transition:
      border-color 0.2s,
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .input:focus {
    outline: none;
    border-color: var(--accent-color, #2196f3);
  }

  .input:disabled {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
    opacity: 0.6;
  }

  .input-hint {
    margin-top: 0.5rem;
    font-size: 0.85rem;
    color: var(--text-secondary, #666);
    transition: color 0.3s ease;
  }
</style>
