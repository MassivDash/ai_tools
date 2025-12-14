<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type {
    ChromaDBCollection,
    ChromaDBResponse
  } from '../../types/chromadb.ts'
  import { collections, selectedCollection } from '../../stores/chromadb.ts'
  import CollectionCard from './CollectionCard.svelte'
  import CreateCollection from './CreateCollection.svelte'
  import IconButton from '../ui/IconButton.svelte'
  import RefreshIcon from '../ui/icons/RefreshIcon.svelte'

  let loading = false
  let error = ''

  const loadCollections = async () => {
    loading = true
    error = ''
    try {
      const response = await axiosBackendInstance.get<
        ChromaDBResponse<ChromaDBCollection[]>
      >('chromadb/collections')
      if (response.data.success && response.data.data) {
        collections.set(response.data.data)

        // Update selectedCollection with latest data if it's still selected
        selectedCollection.update((current) => {
          if (current) {
            // Find the updated collection by id or name
            const updated = response.data.data?.find(
              (c) => c.id === current.id || c.name === current.name
            )
            if (updated) {
              return updated
            }
          }
          // Auto-select the first collection if none is selected and collections exist
          if (!current && response.data.data && response.data.data.length > 0) {
            const firstCollection = response.data.data[0]
            if (firstCollection && firstCollection.name) {
              return firstCollection
            }
          }
          return current
        })
      } else {
        error = response.data.error || 'Failed to load collections'
      }
    } catch (err: any) {
      console.error('❌ Error loading collections:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load collections'
    } finally {
      loading = false
    }
  }

  const handleCollectionSelect = (collection: ChromaDBCollection) => {
    if (!collection || !collection.name) {
      return
    }
    selectedCollection.set(collection)
  }

  const handleCollectionDelete = async (collectionName: string) => {
    if (
      !confirm(
        `Are you sure you want to delete collection "${collectionName}"?`
      )
    ) {
      return
    }

    try {
      const response = await axiosBackendInstance.delete<
        ChromaDBResponse<void>
      >(`chromadb/collections/${collectionName}`)
      if (response.data.success) {
        let wasSelected = false
        selectedCollection.update((current) => {
          wasSelected = current?.name === collectionName
          return current
        })

        // Remove from collections store and handle selection
        collections.update((cols) => {
          const updated = cols.filter((c) => c.name !== collectionName)

          // If the deleted collection was selected, select the first remaining collection
          if (wasSelected) {
            if (updated.length > 0) {
              const firstCollection = updated[0]
              if (firstCollection && firstCollection.name) {
                selectedCollection.set(firstCollection)
              } else {
                selectedCollection.set(null)
              }
            } else {
              selectedCollection.set(null)
            }
          }

          return updated
        })

        // Reload from server to get updated counts
        await loadCollections()
      } else {
        error = response.data.error || 'Failed to delete collection'
      }
    } catch (err: any) {
      console.error('❌ Error deleting collection:', err)
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to delete collection'
    }
  }

  onMount(() => {
    loadCollections()
  })

  // Expose refresh function
  export const refresh = loadCollections
</script>

<div class="collection-list">
  <div class="header">
    <h2>Collections</h2>
    <div class="header-actions">
      <CreateCollection />
      <IconButton
        variant="info"
        onclick={loadCollections}
        disabled={loading}
        title={loading ? 'Loading...' : 'Refresh Collections'}
      >
        <RefreshIcon width="24" height="24" />
      </IconButton>
    </div>
  </div>

  {#if error}
    <div class="error-message">❌ {error}</div>
  {/if}

  {#if loading && $collections.length === 0}
    <div class="loading">Loading collections...</div>
  {:else if $collections.length === 0}
    <div class="empty-state">
      <p>No collections found</p>
      <p class="hint">
        No collections, create one to start your document upload
      </p>
    </div>
  {:else}
    <div class="collections-grid">
      {#each $collections as collection (collection.id)}
        <CollectionCard
          {collection}
          selected={$selectedCollection?.id === collection.id}
          on:select={() => handleCollectionSelect(collection)}
          on:delete={() => handleCollectionDelete(collection.name)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .collection-list {
    width: 100%;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  .header h2 {
    margin: 0;
    font-size: 1.5rem;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .error-message {
    padding: 1rem;
    background: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 4px;
    color: var(--accent-color, #c33);
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .loading {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }

  .empty-state {
    text-align: center;
    padding: 3rem;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }

  .empty-state .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary);
    margin-top: 0.5rem;
    transition: color 0.3s ease;
  }

  .collections-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }

  @media screen and (max-width: 768px) {
    .collections-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
