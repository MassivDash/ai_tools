<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import type { ChromaDBCollection } from '../../types/chromadb.ts'
  import XIcon from '../ui/icons/XIcon.svelte'

  export let collection: ChromaDBCollection
  export let selected: boolean = false

  const dispatch = createEventDispatcher()

  const handleSelect = () => {
    dispatch('select')
  }

  const handleDelete = () => {
    dispatch('delete')
  }
</script>

<div class="collection-card" class:selected onclick={handleSelect}>
  <div class="card-header">
    <h3>{collection.name}</h3>
    <button
      class="delete-btn"
      onclick={(e) => {
        e.stopPropagation()
        handleDelete()
      }}
      title="Delete collection"
    >
      <XIcon width="18" height="18" />
    </button>
  </div>

  <div class="card-body">
    <div class="info-item">
      <span class="label">ID:</span>
      <span class="value">{collection.id}</span>
    </div>

    {#if collection.count !== undefined}
      <div class="info-item">
        <span class="label">Documents:</span>
        <span class="value">{collection.count}</span>
      </div>
    {/if}

    {#if collection.metadata && Object.keys(collection.metadata).length > 0}
      <div class="metadata">
        <span class="label">Metadata:</span>
        <div class="metadata-items">
          {#each Object.entries(collection.metadata) as [key, value]}
            <div class="metadata-item">
              <strong>{key}:</strong>
              {value}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .collection-card {
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

  .collection-card:hover {
    border-color: var(--border-color-hover);
    box-shadow: 0 4px 8px var(--shadow);
    transform: translateY(-2px);
  }

  .collection-card.selected {
    border-color: var(--accent-color, #4a90e2);
    background: var(--bg-secondary);
    box-shadow: 0 4px 12px rgba(74, 144, 226, 0.2);
  }

  .collection-card.selected:hover {
    border-color: var(--accent-color, #4a90e2);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .card-header h3 {
    margin: 0;
    font-size: 1.2rem;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .delete-btn {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    cursor: pointer;
    padding: 0.4rem;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.8;
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
  }

  .metadata {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-color);
    transition: border-color 0.3s ease;
  }

  .metadata-items {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-top: 0.5rem;
  }

  .metadata-item {
    font-size: 0.9rem;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }
</style>
