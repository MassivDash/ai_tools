<script lang="ts">
  import Button from './Button.svelte'

  interface Props {
    variant?: 'primary' | 'secondary' | 'success' | 'danger' | 'info' | 'ghost'
    disabled?: boolean
    title?: string
    iconSize?: number | string
    class?: string
    onclick?: (_e: MouseEvent) => void
    children?: import('svelte').Snippet
    [key: string]: any
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
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    border-radius: 8px !important;
  }

  :global(.button-icon-only) :global(svg) {
    flex-shrink: 0;
    width: var(--icon-size, 24px);
    height: var(--icon-size, 24px);
  }
</style>
