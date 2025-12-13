<script lang="ts">
  interface Props {
    title: string
    open?: boolean
    children?: import('svelte').Snippet
  }

  let { title, open = false, children }: Props = $props()
  let isOpen = $state(false)

  // Sync external open prop changes to internal state
  $effect(() => {
    isOpen = open
  })

  const toggle = () => {
    isOpen = !isOpen
  }
</script>

<div class="accordion">
  <button class="accordion-header" onclick={toggle} aria-expanded={isOpen}>
    <span class="accordion-title">{title}</span>
    <span class="accordion-icon" class:open={isOpen}>▼</span>
  </button>
  {#if isOpen}
    <div class="accordion-content">
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</div>

<style>
  .accordion {
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 4px;
    margin-bottom: 1rem;
    background-color: var(--bg-primary, #fff);
    transition: border-color 0.3s ease, background-color 0.3s ease;
  }

  .accordion-header {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    background-color: var(--bg-secondary, #f9f9f9);
    transition: background-color 0.2s ease;
  }

  .accordion-header:hover {
    background-color: var(--bg-tertiary, #f0f0f0);
  }

  .accordion-title {
    font-weight: 600;
    color: var(--text-primary, #333);
    font-size: 1rem;
    transition: color 0.3s ease;
  }

  .accordion-icon {
    transition: transform 0.2s ease, color 0.3s ease;
    color: var(--text-secondary, #666);
    font-size: 0.75rem;
  }

  .accordion-icon.open {
    transform: rotate(180deg);
  }

  .accordion-content {
    padding: 1rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    transition: border-color 0.3s ease;
  }
</style>

