<script lang="ts">
  import Button from './Button.svelte'

  interface Props {
    variant?: 'primary' | 'secondary' | 'success' | 'danger' | 'info'
    disabled?: boolean
    title?: string
    iconSize?: number | string
    class?: string
    onclick?: (_e: MouseEvent) => void
    children?: import('svelte').Snippet
  }

  let {
    variant = 'info',
    disabled = false,
    title = '',
    iconSize = 32,
    class: extraClass = '',
    onclick,
    children,
    ...restProps
  }: Props = $props()

  const iconSizeValue = $derived(
    typeof iconSize === 'number' ? `${iconSize}px` : iconSize
  )
</script>

<Button
  {variant}
  {disabled}
  class="button-icon-only {extraClass}"
  {title}
  {onclick}
  style="--icon-size: {iconSizeValue}"
  {...restProps}
>
  {#if children}
    {@render children()}
  {/if}
</Button>

<style>
  :global(.button-icon-only) {
    padding: 0.75rem !important;
    min-width: 3rem !important;
    min-height: 3rem !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    border-radius: 8px !important;
  }

  :global(.button-icon-only) :global(svg) {
    flex-shrink: 0;
    width: var(--icon-size, 32px);
    height: var(--icon-size, 32px);
  }
</style>
