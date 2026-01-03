<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance'
  import SidebarHeader from '@ui/SidebarHeader.svelte'
  import type { Conversation } from '@types'
  import ConversationList from './components/ConversationList.svelte'

  export let currentConversationId: string | undefined
  export let isOpen = false
  export let shouldRefresh = false

  export let onSelect: ((id: string) => void) | undefined = undefined
  export let onNew: (() => void) | undefined = undefined
  export let onClose: (() => void) | undefined = undefined

  $: if (shouldRefresh) {
    loadConversations()
  }

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
    if (onSelect) onSelect(id)
    if (window.innerWidth < 768) {
      if (onClose) onClose()
    }
  }

  const handleNewChat = () => {
    if (onNew) onNew()
    if (window.innerWidth < 768) {
      if (onClose) onClose()
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
    on:close={() => {
      if (onClose) onClose()
    }}
  />

  <div class="content">
    <ConversationList
      {conversations}
      activeId={currentConversationId}
      {loading}
      on:select={(e) => selectConversation(e.detail.id)}
      on:save={(e) => saveTitle(e.detail.id, e.detail.title)}
      on:delete={(e) => deleteConversation(e.detail.id)}
    />
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
    display: flex;
    flex-direction: column;
  }

  :global(.sidebar-icon-btn.button-icon-only) {
    min-width: 2rem !important;
    min-height: 2rem !important;
    padding: 0 !important;
    width: 2rem;
    height: 2rem;
  }
</style>
