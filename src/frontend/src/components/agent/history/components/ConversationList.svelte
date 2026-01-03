<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import EditableListItem from '@ui/EditableListItem.svelte'
  import type { Conversation } from '@types'

  export let conversations: Conversation[] = []
  export let activeId: string | undefined = undefined
  export let loading = false

  const dispatch = createEventDispatcher<{
    select: { id: string }
    save: { id: string; title: string }
    delete: { id: string }
  }>()
</script>

<div class="list-container">
  {#if loading}
    <div class="loading">Loading...</div>
  {:else if conversations.length === 0}
    <div class="empty">No history yet</div>
  {:else}
    <div class="list">
      {#each conversations as conv (conv.id)}
        <EditableListItem
          title={conv.title || 'New Conversation'}
          model={conv.model}
          active={activeId === conv.id}
          on:click={() => dispatch('select', { id: conv.id })}
          on:save={(e) => dispatch('save', { id: conv.id, title: e.detail })}
          on:delete={() => dispatch('delete', { id: conv.id })}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .list-container {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .list {
    display: flex;
    flex-direction: column;
  }

  .loading,
  .empty {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }
</style>
