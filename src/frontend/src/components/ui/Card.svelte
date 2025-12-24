<script lang="ts">
  interface Props {
    variant?: 'elevated' | 'filled' | 'outlined'
    class?: string
    children?: import('svelte').Snippet
    [key: string]: any
  }

  let {
    variant = 'elevated',
    class: className = '',
    children,
    ...restProps
  }: Props = $props()

  const classes = $derived(`card card-${variant} ${className}`.trim())
</script>

<div class={classes} {...restProps}>
  {#if children}
    {@render children()}
  {/if}
</div>

<style>
  .card {
    border-radius: 12px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    transition: all 0.2s ease-in-out;
  }

  /* Elevated Card (Default) */
  .card-elevated {
    background-color: var(--md-surface);
    color: var(--md-on-surface);
    box-shadow: 0 2px 8px -2px var(--md-shadow);
  }

  .card-elevated:hover {
    box-shadow: 0 4px 16px -4px var(--md-shadow);
  }

  /* Filled Card */
  .card-filled {
    background-color: var(--md-surface-variant);
    color: var(--md-on-surface-variant);
    border: none;
  }

  /* Outlined Card */
  .card-outlined {
    background-color: var(--md-surface);
    color: var(--md-on-surface);
    border: 1px solid var(--md-outline-variant);
  }
</style>
