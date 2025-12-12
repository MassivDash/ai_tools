<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'success' | 'danger' | 'info' =
    'primary'
  export let disabled: boolean = false
  export let type: 'button' | 'submit' | 'reset' = 'button'
  export let size: 'small' | 'medium' | 'large' = 'medium'

  $: className = `button button-${variant} button-${size}`
</script>

<button {type} {disabled} class={className} on:click {...$$restProps}>
  <slot />
</button>

<style>
  .button {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
    font-weight: 600;
    font-family: inherit;
  }

  .button-small {
    padding: 0.375rem 0.75rem;
    font-size: 0.85rem;
  }

  .button-medium {
    padding: 0.5rem 1rem;
    font-size: 0.9rem;
  }

  .button-large {
    padding: 0.75rem 1.5rem;
    font-size: 1rem;
  }

  .button-primary {
    background-color: #2196f3;
    color: white;
  }

  .button-primary:hover:not(:disabled) {
    background-color: #1976d2;
  }

  .button-secondary {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--text-secondary, #666);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-secondary:hover:not(:disabled) {
    background-color: var(--bg-tertiary, #e8e8e8);
    color: var(--text-primary, #333);
  }

  .button-success {
    background-color: #4caf50;
    color: white;
  }

  .button-success:hover:not(:disabled) {
    background-color: #45a049;
  }

  .button-danger {
    background-color: #f44336;
    color: white;
  }

  .button-danger:hover:not(:disabled) {
    background-color: #da190b;
  }

  .button-info {
    background-color: #2196f3;
    color: white;
  }

  .button-info:hover:not(:disabled) {
    background-color: #1976d2;
  }

  .button:disabled {
    background-color: var(--text-tertiary, #ccc);
    color: var(--text-secondary, #666);
    cursor: not-allowed;
    opacity: 0.6;
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }
</style>
