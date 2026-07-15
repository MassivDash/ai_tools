<script lang="ts">
  import { onMount } from 'svelte'
  import SearchableList from '@ui/SearchableList.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import CheckboxWithHelp from '@ui/CheckboxWithHelp.svelte'
  import LabelWithHelp from '@ui/LabelWithHelp.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { Collection } from '@types'

  export let chromadbEnabled: boolean = false
  export let collections: Collection[] = []
  export let selectedCollection: string = ''
  export let loadingCollections: boolean = false
  export let onToggle: () => void
  export let onCollectionSelect: (_collection: Collection) => void

  const getCollectionKey = (collection: Collection) => collection.id
  const getCollectionLabel = (collection: Collection) => collection.name
  const getCollectionSubtext = (collection: Collection) => {
    const parts = []
    if (collection.count !== undefined) {
      parts.push(`${collection.count} documents`)
    }
    return parts.join(' • ')
  }
</script>

<div class="config-section">
  <div class="chromadb-card">
    <div class="card-header">
      <MaterialIcon name="database" width="20" height="20" />
      <span>Knowledge Base</span>
    </div>

    <div class="card-content">
      <div class="enable-row" class:has-content={chromadbEnabled}>
        <CheckboxWithHelp
          bind:checked={chromadbEnabled}
          onchange={onToggle}
          label="Enable ChromaDB"
          helpText="Enable ChromaDB to allow the agent to search your knowledge base collections for relevant information."
        />
      </div>

      {#if chromadbEnabled}
        <div class="config-settings">
          <!-- Collection Selection -->
          <div class="config-subsection">
            <LabelWithHelp
              id="collection"
              label="Collection"
              helpText="Select the ChromaDB collection to use for searches. The agent will query this collection when it needs information."
            />
            {#if loadingCollections}
              <div class="loading">Loading collections...</div>
            {:else if collections.length > 0}
              <SearchableList
                items={collections}
                searchPlaceholder="Search collections..."
                emptyMessage="No collections found"
                getItemKey={getCollectionKey}
                getItemLabel={getCollectionLabel}
                getItemSubtext={getCollectionSubtext}
                selectedKey={(() => {
                  const selected = collections.find(
                    (c) => c.name === selectedCollection
                  )
                  return selected ? selected.id : null
                })()}
                onselect={onCollectionSelect}
              />
            {:else}
              <div class="no-items">
                <p>No collections found</p>
                <p class="hint-small">
                  Create a collection in the ChromaDB manager first
                </p>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .config-section {
    margin-bottom: 2rem;
  }

  .chromadb-card {
    background: var(--bg-primary, #ffffff);
    border-radius: 8px;
    border: 1px solid var(--border-color, #e0e0e0);
    overflow: hidden;
    transition:
      box-shadow 0.2s ease,
      border-color 0.2s ease;
  }

  .chromadb-card:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
    border-color: var(--accent-color-alpha, rgba(33, 150, 243, 0.3));
  }

  .card-header {
    margin: 0;
    padding: 0.75rem 1rem;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-primary);
    font-weight: 600;
    background-color: var(--bg-secondary, #f8f9fa);
    border-bottom: 1px solid var(--border-color, #e0e0e0);
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .card-content {
    display: flex;
    flex-direction: column;
  }

  .enable-row {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid transparent;
    transition: background-color 0.15s ease;
  }

  .enable-row:not(:last-child) {
    border-bottom: 1px solid var(--border-color-light, #f0f0f0);
  }

  /* Apply border separator if content follows */
  .enable-row.has-content {
    border-bottom: 1px solid var(--border-color, #e0e0e0);
  }

  .enable-row:hover {
    background-color: var(--bg-tertiary, #fafafa);
  }

  .config-subsection {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-color-light, #f0f0f0);
  }

  .config-subsection:last-child {
    border-bottom: none;
  }

  .loading,
  .no-items {
    padding: 1rem;
    text-align: center;
    color: var(--text-secondary);
    background: var(--bg-tertiary, #f8f9fa);
    border-radius: 6px;
  }

  .no-items .hint-small {
    font-size: 0.85rem;
    color: var(--text-tertiary);
    margin-top: 0.5rem;
  }
</style>
