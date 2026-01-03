<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance'

  import EditableListItem from '@ui/EditableListItem.svelte'
  import SidebarHeader from '@ui/SidebarHeader.svelte'
  import type { Conversation } from '@types'

  export let currentConversationId: string | undefined
  export let isOpen = false
  export let shouldRefresh = false

  $: if (shouldRefresh) {
    loadConversations()
  }

  const dispatch = createEventDispatcher<{
    select: string
    new: void
    close: void
  }>()

  let conversations: Conversation[] = []
  let loading = false

  const loadConversations = async () => {
    loading = true
    try {
      const response = await axiosBackendInstance.get<Conversation[]>(
        'agent/conversations'
      )
      conversations = response.data
    } catch (err: any) {
      console.error('Failed to load conversations:', err)
    } finally {
      loading = false
    }
  }

  const selectConversation = (id: string) => {
    dispatch('select', id)
    if (window.innerWidth < 768) {
      dispatch('close') // Auto close on mobile
    }
  }

  const handleNewChat = () => {
    dispatch('new')
    if (window.innerWidth < 768) {
      dispatch('close')
    }
  }

  const saveTitle = async (id: string, newTitle: string) => {
    try {
      await axiosBackendInstance.patch(`agent/conversations/${id}`, {
        title: newTitle
      })
      // Update local state
      conversations = conversations.map((c) =>
        c.id === id ? { ...c, title: newTitle } : c
      )
    } catch (err) {
      console.error('Failed to update title:', err)
    }
  }

  const deleteConversation = async (id: string) => {
    try {
      await axiosBackendInstance.delete(`agent/conversations/${id}`)
      conversations = conversations.filter((c) => c.id !== id)
      if (currentConversationId === id) {
        handleNewChat()
      }
    } catch (err) {
      console.error('Failed to delete conversation:', err)
    }
  }

  onMount(() => {
    loadConversations()
  })

  // Refresh list when opened
  $: if (isOpen) {
    loadConversations()
  }
</script>

<div class="history-sidebar" class:open={isOpen}>
  <SidebarHeader
    title="History"
    icon="history"
    addTitle="New Chat"
    closeTitle="Close History"
    on:add={handleNewChat}
    on:close={() => dispatch('close')}
  />

  <div class="content">
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
            active={currentConversationId === conv.id}
            on:click={() => selectConversation(conv.id)}
            on:save={(e) => saveTitle(conv.id, e.detail)}
            on:delete={() => deleteConversation(conv.id)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .history-sidebar {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    width: 260px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    transform: translateX(-100%);
    transition: transform 0.3s ease;
    border-top-right-radius: 8px;
    border-bottom-right-radius: 8px;
    z-index: 20;
    display: flex;
    flex-direction: column;
  }

  .history-sidebar.open {
    transform: translateX(0);
  }

  .content {
    flex: 1;
    overflow-y: auto;
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

  :global(.sidebar-icon-btn.button-icon-only) {
    min-width: 2rem !important;
    min-height: 2rem !important;
    padding: 0 !important;
    width: 2rem;
    height: 2rem;
  }
</style>
