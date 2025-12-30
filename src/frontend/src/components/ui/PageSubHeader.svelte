<script lang="ts">
  interface Props {
    title: string
    actions?: import('svelte').Snippet
    leftContent?: import('svelte').Snippet
  }

  let { title, actions, leftContent }: Props = $props()
</script>

<div class="page-sub-header">
  <div class="header-left">
    <h3>{title}</h3>
    {#if leftContent}
      {@render leftContent()}
    {/if}
  </div>
  {#if actions}
    <div class="header-actions">
      {@render actions()}
    </div>
  {/if}
</div>

<style>
  .page-sub-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    transition: background-color 0.3s ease;
    max-width: 1560px;
    width: 100%;
    height: 75px;
    margin: 0 auto;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex: 1;
    min-width: 0;
  }

  .page-sub-header h3 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    font-size: 1.5rem;
    transition: color 0.3s ease;
    white-space: nowrap;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .header-actions :global(.button-icon-only) {
    padding: 0.75rem !important;
    min-width: 3rem !important;
    min-height: 3rem !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .header-actions :global(.button-icon-only) :global(svg) {
    flex-shrink: 0;
  }
</style>
