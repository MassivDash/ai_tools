<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import type { PageIndexDocument } from '@types'
  import MaterialIcon from '@ui/MaterialIcon.svelte'

  export let document: PageIndexDocument
  export let selected: boolean = false

  const dispatch = createEventDispatcher()

  const handleSelect = () => {
    dispatch('select')
  }

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      handleSelect()
    }
  }

  const handleDelete = () => {
    dispatch('delete')
  }

  const formatDate = (timestamp: number): string => {
    // created_at is assumed to be a unix timestamp in seconds
    const date = new Date(timestamp * 1000)
    if (isNaN(date.getTime())) return ''
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    })
  }
</script>

<div
  class="document-card"
  class:selected
  role="button"
  tabindex="0"
  onclick={handleSelect}
  onkeydown={handleKeydown}
  aria-label={`Select document ${document.title}`}
>
  <div class="card-header">
    <h3>{document.title}</h3>
    <button
      class="delete-btn"
      onclick={(e) => {
        e.stopPropagation()
        handleDelete()
      }}
      title="Delete document"
    >
      <MaterialIcon name="close" width="18" height="18" />
    </button>
  </div>

  <div class="card-body">
    <div class="info-item">
      <span class="label">File:</span>
      <span class="value">{document.filename}</span>
    </div>

    <div class="status-row">
      <span
        class="status-badge"
        class:processing={document.status === 'processing'}
        class:ready={document.status === 'ready'}
        class:error={document.status === 'error'}
      >
        {document.status}
      </span>
      {#if document.created_at}
        <span class="date">{formatDate(document.created_at)}</span>
      {/if}
    </div>

    {#if document.status === 'ready'}
      <div class="info-item">
        <span class="label">Pages:</span>
        <span class="value">{document.page_count ?? '—'}</span>
      </div>
      <div class="info-item">
        <span class="label">Sections:</span>
        <span class="value">{document.node_count ?? '—'}</span>
      </div>
    {/if}

    {#if document.status === 'error' && document.error}
      <div class="error-text">{document.error}</div>
    {/if}
  </div>
</div>

<style>
  .document-card {
    background: var(--bg-primary);
    border: 2px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
    cursor: pointer;
    transition:
      all 0.2s ease,
      background-color 0.3s ease,
      border-color 0.3s ease;
    box-shadow: 0 2px 4px var(--shadow);
  }

  .document-card:hover {
    border-color: var(--border-color-hover);
    box-shadow: 0 4px 8px var(--shadow);
    transform: translateY(-2px);
  }

  .document-card.selected {
    border-color: var(--accent-color, #4a90e2);
    background: var(--bg-secondary);
    box-shadow: 0 4px 12px rgba(74, 144, 226, 0.2);
  }

  .document-card.selected:hover {
    border-color: var(--accent-color, #4a90e2);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    gap: 0.5rem;
  }

  .card-header h3 {
    margin: 0;
    font-size: 1.2rem;
    color: var(--text-primary);
    transition: color 0.3s ease;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .delete-btn {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    cursor: pointer;
    padding: 0.4rem;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.8;
    flex-shrink: 0;
    transition:
      opacity 0.2s,
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
    color: var(--text-primary);
  }

  .delete-btn:hover {
    opacity: 1;
    background: var(--bg-tertiary);
    border-color: var(--border-color-hover);
    color: var(--accent-color, #c33);
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .info-item {
    display: flex;
    gap: 0.5rem;
  }

  .label {
    font-weight: 600;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }

  .value {
    color: var(--text-primary);
    transition: color 0.3s ease;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .status-badge {
    display: inline-block;
    padding: 0.2rem 0.6rem;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 600;
    text-transform: capitalize;
  }

  .status-badge.processing {
    background: rgba(33, 150, 243, 0.15);
    color: #2196f3;
  }

  .status-badge.ready {
    background: rgba(76, 175, 80, 0.15);
    color: #4caf50;
  }

  .status-badge.error {
    background: rgba(244, 67, 54, 0.15);
    color: #f44336;
  }

  .date {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .error-text {
    font-size: 0.85rem;
    color: #f44336;
    word-break: break-word;
  }
</style>
