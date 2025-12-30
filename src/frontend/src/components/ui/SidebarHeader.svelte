<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import MaterialIcon from './MaterialIcon.svelte'
  import Button from './Button.svelte'

  export let title: string
  export let icon: string = ''
  export let showAdd: boolean = true
  export let showClose: boolean = true
  export let addTitle: string = 'Add'
  export let closeTitle: string = 'Close'

  const dispatch = createEventDispatcher<{
    add: void
    close: void
  }>()
</script>

<div class="header">
  <div class="header-left">
    <slot name="prefix">
      {#if icon}
        <MaterialIcon name={icon} width="24" height="24" />
      {/if}
    </slot>
    <h2>{title}</h2>
  </div>
  <div class="actions">
    {#if showAdd}
      <Button
        variant="info"
        class="sidebar-icon-btn button-icon-only"
        onclick={() => dispatch('add')}
        title={addTitle}
      >
        <MaterialIcon name="plus" width="20" height="20" />
      </Button>
    {/if}
    <slot name="actions" />
    {#if showClose}
      <Button
        variant="info"
        class="sidebar-icon-btn button-icon-only"
        onclick={() => dispatch('close')}
        title={closeTitle}
      >
        <MaterialIcon name="chevron-left" width="20" height="20" />
      </Button>
    {/if}
  </div>
</div>

<style>
  .header {
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    overflow: hidden;
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  :global(.sidebar-icon-btn.button-icon-only) {
    min-width: 2rem !important;
    min-height: 2rem !important;
    padding: 0 !important;
    width: 2rem;
    height: 2rem;
  }
</style>
