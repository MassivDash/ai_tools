<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import type { Conversation } from './types'

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
  let error = ''
  let editingId: string | null = null
  let editTitle = ''
  let deleteConfirmId: string | null = null

  const loadConversations = async () => {
    loading = true
    try {
      const response = await axiosBackendInstance.get<Conversation[]>(
        'agent/conversations'
      )
      conversations = response.data
    } catch (err: any) {
      console.error('Failed to load conversations:', err)
      error = 'Failed to load history'
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

  const startEdit = (conv: Conversation, event: Event) => {
    event.stopPropagation()
    editingId = conv.id
    editTitle = conv.title || 'New Conversation'
  }

  const saveTitle = async () => {
    if (!editingId) return
    try {
      await axiosBackendInstance.patch(`agent/conversations/${editingId}`, {
        title: editTitle
      })
      // Update local state
      conversations = conversations.map((c) =>
        c.id === editingId ? { ...c, title: editTitle } : c
      )
      editingId = null
    } catch (err) {
      console.error('Failed to update title:', err)
    }
  }

  const deleteConversation = async (id: string) => {
    try {
      await axiosBackendInstance.delete(`agent/conversations/${id}`)
      conversations = conversations.filter((c) => c.id !== id)
      deleteConfirmId = null
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
  <div class="header">
    <h2>History</h2>
    <div class="actions">
      <button class="new-chat-btn" on:click={handleNewChat} title="New Chat">
        <MaterialIcon name="plus" width="24" height="24" />
      </button>
      <button
        class="close-btn"
        on:click={() => dispatch('close')}
        title="Close History"
      >
        <MaterialIcon name="chevron-left" width="24" height="24" />
      </button>
    </div>
  </div>

  <div class="content">
    {#if loading}
      <div class="loading">Loading...</div>
    {:else if conversations.length === 0}
      <div class="empty">No history yet</div>
    {:else}
      <div class="list">
        {#each conversations as conv (conv.id)}
          <div
            class="item"
            class:active={currentConversationId === conv.id}
            on:click={() => selectConversation(conv.id)}
            on:keypress={(e) =>
              e.key === 'Enter' && selectConversation(conv.id)}
            role="button"
            tabindex="0"
          >
            {#if editingId === conv.id}
              <input
                type="text"
                bind:value={editTitle}
                on:click|stopPropagation
                on:keypress|stopPropagation={(e) => {
                  if (e.key === 'Enter') saveTitle()
                }}
                on:blur={saveTitle}
                autofocus
              />
            {:else if deleteConfirmId === conv.id}
              <div class="confirm-delete">
                <span>Delete?</span>
                <button
                  class="confirm-btn"
                  on:click|stopPropagation={() => deleteConversation(conv.id)}
                >
                  Yes
                </button>
                <button
                  class="cancel-btn"
                  on:click|stopPropagation={() => {
                    deleteConfirmId = null
                  }}
                >
                  No
                </button>
              </div>
            {:else}
              <span class="title" title={conv.title || 'New Conversation'}>
                {conv.title || 'New Conversation'}
              </span>
              <div class="item-actions">
                <button
                  class="action-btn"
                  on:click|stopPropagation={(e) => startEdit(conv, e)}
                  title="Rename"
                >
                  <MaterialIcon name="pencil" width="18" height="18" />
                </button>
                <button
                  class="action-btn delete"
                  on:click|stopPropagation={() => {
                    deleteConfirmId = conv.id
                  }}
                  title="Delete"
                >
                  <MaterialIcon name="delete" width="18" height="18" />
                </button>
              </div>
            {/if}
          </div>
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
    background: var(--bg-secondary, #f5f5f5);
    border-right: 1px solid var(--border-color, #e0e0e0);
    transform: translateX(-100%);
    transition: transform 0.3s ease;
    z-index: 20;
    display: flex;
    flex-direction: column;
  }

  .history-sidebar.open {
    transform: translateX(0);
  }

  .header {
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color, #e0e0e0);
  }

  .header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary, #333);
  }

  .content {
    flex: 1;
    overflow-y: auto;
  }

  .new-chat-btn,
  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary, #666);
    padding: 4px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .new-chat-btn:hover,
  .close-btn:hover {
    background: rgba(0, 0, 0, 0.05);
    color: var(--primary-color, #2196f3);
  }

  .list {
    display: flex;
    flex-direction: column;
  }

  .item {
    padding: 0.75rem 1rem;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-light, #eee);
    transition: background 0.2s;
    height: 3rem;
  }

  .item:hover {
    background: rgba(0, 0, 0, 0.03);
  }

  .item.active {
    background: rgba(33, 150, 243, 0.1);
    border-left: 3px solid var(--primary-color, #2196f3);
  }

  .title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.9rem;
    color: var(--text-primary, #333);
    margin-right: 8px;
  }

  .item-actions {
    display: flex; /* Always visible but low opacity maybe? or visible on hover */
    gap: 4px;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .item:hover .item-actions {
    opacity: 1;
  }

  .action-btn {
    background: none;
    border: none;
    padding: 2px;
    cursor: pointer;
    color: var(--text-secondary, #999);
    display: flex;
    align-items: center;
  }

  .action-btn:hover {
    color: var(--primary-color, #2196f3);
  }

  .action-btn.delete:hover {
    color: #f44336;
  }

  .confirm-delete {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    font-size: 0.85rem;
  }

  .confirm-btn {
    background: #f44336;
    color: white;
    border: none;
    border-radius: 8px;
    padding: 2px 8px;
    cursor: pointer;
  }

  .cancel-btn {
    background: #ccc;
    color: #333;
    border: none;
    border-radius: 8px;
    padding: 2px 8px;
    cursor: pointer;
  }

  input[type='text'] {
    width: 100%;
    padding: 4px;
    border: 1px solid var(--primary-color, #2196f3);
    border-radius: 8px;
    outline: none;
    font-family: inherit;
    font-size: 0.9rem;
  }

  .loading,
  .empty {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary, #999);
    font-size: 0.9rem;
  }
</style>
