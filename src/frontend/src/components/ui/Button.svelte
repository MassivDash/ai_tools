<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'success' | 'danger' | 'info' =
    'primary'
  export let disabled: boolean = false
  export let type: 'button' | 'submit' | 'reset' = 'button'
  export let size: 'small' | 'medium' | 'large' = 'medium'

  let extraClass = ''
  export { extraClass as class }

  $: classes =
    `button button-${variant} button-${size} ${extraClass || ''}`.trim()
</script>

<button {type} {disabled} class={classes} on:click {...$$restProps}>
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
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    justify-content: center;
  }

  .button.button-icon-only {
    padding: 0.75rem !important;
    min-width: 3rem !important;
    min-height: 3rem !important;
  }

  .button-success.button-icon-only {
    background-color: #4caf50 !important;
    color: white !important;
  }

  .button-success.button-icon-only:hover:not(:disabled) {
    background-color: #45a049 !important;
  }

  .button-danger.button-icon-only {
    background-color: #f44336 !important;
    color: white !important;
  }

  .button-danger.button-icon-only:hover:not(:disabled) {
    background-color: #da190b !important;
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
    background-color: var(--accent-color, #b12424);
    color: white;
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-primary:hover:not(:disabled) {
    background-color: var(--accent-hover, #8f1d1d);
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
    background-color: #4caf50 !important;
    color: white !important;
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-success:hover:not(:disabled) {
    background-color: #45a049 !important;
  }

  .button-danger {
    background-color: #f44336 !important;
    color: white !important;
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-danger:hover:not(:disabled) {
    background-color: #da190b !important;
  }

  .button-info {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--text-primary, #333);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-info:hover:not(:disabled) {
    background-color: var(--bg-tertiary, #e8e8e8);
    color: var(--text-primary, #333);
  }

  .button-success:disabled,
  .button-danger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .button:disabled:not(.button-success):not(.button-danger) {
    background-color: var(--text-tertiary, #ccc);
    color: var(--text-secondary, #666);
    cursor: not-allowed;
    opacity: 0.6;
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }
</style>
