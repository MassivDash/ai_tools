<script lang="ts">
  import Button from './Button.svelte'
  
  export let variant: 'primary' | 'secondary' | 'success' | 'danger' | 'info' = 'info'
  export let disabled: boolean = false
  export let title: string = ''
  export let iconSize: number | string = 32
</script>

<Button
  {variant}
  {disabled}
  class="button-icon-only"
  {title}
  {...$$restProps}
>
  <slot />
</Button>

<style>
  :global(.button-icon-only) {
    padding: 0.75rem !important;
    min-width: 3rem !important;
    min-height: 3rem !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  :global(.button-icon-only) :global(svg) {
    flex-shrink: 0;
  }
</style>

