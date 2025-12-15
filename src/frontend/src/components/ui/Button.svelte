<script lang="ts">
  interface Props {
    variant?: 'primary' | 'secondary' | 'success' | 'danger' | 'info'
    disabled?: boolean
    type?: 'button' | 'submit' | 'reset'
    size?: 'small' | 'medium' | 'large'
    class?: string
    children?: import('svelte').Snippet
    onclick?: (_e: MouseEvent) => void
    title?: string
  }

  let {
    variant = 'primary',
    disabled = false,
    type = 'button',
    size = 'medium',
    title = '',
    class: extraClass = '',
    children,
    onclick,
    ...restProps
  }: Props = $props()

  const classes = $derived(
    `button button-${variant} button-${size} ${extraClass || ''}`.trim()
  )
</script>

<button {type} {disabled} class={classes} {onclick} {title} {...restProps}>
  {#if children}
    {@render children()}
  {/if}
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
    background-color: var(--md-tertiary) !important;
    color: var(--md-on-tertiary) !important;
  }

  .button-success.button-icon-only:hover:not(:disabled) {
    background-color: var(--md-tertiary-container) !important;
    color: var(--md-on-tertiary-container) !important;
  }

  .button-danger.button-icon-only {
    background-color: var(--md-error) !important;
    color: var(--md-on-error) !important;
  }

  .button-danger.button-icon-only:hover:not(:disabled) {
    background-color: var(--md-error-container) !important;
    color: var(--md-on-error-container) !important;
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
    background-color: var(--md-primary);
    color: var(--md-on-primary);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-primary:hover:not(:disabled) {
    background-color: var(--accent-hover);
  }

  .button-secondary {
    background-color: var(--md-secondary-container);
    color: var(--md-on-secondary-container);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-secondary:hover:not(:disabled) {
    background-color: var(--md-surface-variant);
    color: var(--md-on-surface-variant);
  }

  .button-success {
    background-color: var(--md-tertiary);
    color: var(--md-on-tertiary);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-success:hover:not(:disabled) {
    background-color: var(--md-tertiary-container);
    color: var(--md-on-tertiary-container);
  }

  .button-danger {
    background-color: var(--md-error);
    color: var(--md-on-error);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-danger:hover:not(:disabled) {
    background-color: var(--md-error-container);
    color: var(--md-on-error-container);
  }

  .button-info {
    background-color: var(--md-surface-variant);
    color: var(--md-on-surface-variant);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .button-info:hover:not(:disabled) {
    background-color: var(--md-secondary-container);
    color: var(--md-on-secondary-container);
  }

  .button-success:disabled,
  .button-danger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .button:disabled:not(.button-success):not(.button-danger) {
    background-color: var(--md-outline-variant);
    color: var(--md-on-surface-variant);
    cursor: not-allowed;
    opacity: 0.6;
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }
</style>
