<script lang="ts">
  import MaterialIcon from './MaterialIcon.svelte'

  interface Props {
    title: string
    icon?: string
    actions?: import('svelte').Snippet
    leftContent?: import('svelte').Snippet
  }

  let { title, icon, actions, leftContent }: Props = $props()
</script>

<div class="page-sub-header">
  <div class="header-left">
    {#if icon}
      <MaterialIcon name={icon} width="32" height="32" />
    {/if}
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
    transition: background-color 0.3s ease;
    width: 100%;
    min-height: 75px;
    margin: 0 auto;
    flex-wrap: wrap;
    gap: 1rem;
    max-width: calc(100% - 4rem);
  }

  @media (max-width: 768px) {
    .page-sub-header {
      padding: 0.75rem;
      gap: 0.75rem;
    }
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

  @media (max-width: 768px) {
    .page-sub-header h3 {
      white-space: normal;
      font-size: 1.25rem;
    }
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
