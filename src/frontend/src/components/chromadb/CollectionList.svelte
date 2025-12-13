<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ChromaDBCollection, ChromaDBResponse } from '../../types/chromadb.ts'
  import CollectionCard from './CollectionCard.svelte'
  import CreateCollection from './CreateCollection.svelte'
  import IconButton from '../ui/IconButton.svelte'
  import RefreshIcon from '../ui/icons/RefreshIcon.svelte'

  const dispatch = createEventDispatcher()

  let collections: ChromaDBCollection[] = []
  let loading = false
  let error = ''
  let selectedCollection: ChromaDBCollection | null = null

  const loadCollections = async () => {
    loading = true
    error = ''
    try {
      console.log('📚 Loading collections...')
      const response = await axiosBackendInstance.get<ChromaDBResponse<ChromaDBCollection[]>>(
        'chromadb/collections'
      )
      if (response.data.success && response.data.data) {
        collections = response.data.data
        console.log('✅ Loaded collections:', collections)
      } else {
        error = response.data.error || 'Failed to load collections'
      }
    } catch (err: any) {
      console.error('❌ Error loading collections:', err)
      error = err.response?.data?.error || err.message || 'Failed to load collections'
    } finally {
      loading = false
    }
  }

  const handleCollectionSelect = (collection: ChromaDBCollection) => {
    selectedCollection = collection
    dispatch('select', collection)
    console.log('📌 Selected collection:', collection)
  }

  const handleCollectionDelete = async (collectionName: string) => {
    if (!confirm(`Are you sure you want to delete collection "${collectionName}"?`)) {
      return
    }

    try {
      console.log('🗑️ Deleting collection:', collectionName)
      const response = await axiosBackendInstance.delete<ChromaDBResponse<void>>(
        `chromadb/collections/${collectionName}`
      )
      if (response.data.success) {
        console.log('✅ Collection deleted')
        await loadCollections() // Reload list
      } else {
        error = response.data.error || 'Failed to delete collection'
      }
    } catch (err: any) {
      console.error('❌ Error deleting collection:', err)
      error = err.response?.data?.error || err.message || 'Failed to delete collection'
    }
  }

  const handleCollectionCreated = () => {
    loadCollections() // Reload list when collection is created
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
      <CreateCollection on:created={handleCollectionCreated} />
      <IconButton variant="info" onclick={loadCollections} disabled={loading} title={loading ? 'Loading...' : 'Refresh Collections'}>
        <RefreshIcon width="24" height="24" />
      </IconButton>
    </div>
  </div>

  {#if error}
    <div class="error-message">❌ {error}</div>
  {/if}

  {#if loading && collections.length === 0}
    <div class="loading">Loading collections...</div>
  {:else if collections.length === 0}
    <div class="empty-state">
      <p>No collections found</p>
      <p class="hint">Create a collection to get started</p>
    </div>
  {:else}
    <div class="collections-grid">
      {#each collections as collection (collection.id)}
        <CollectionCard
          {collection}
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
    transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease;
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

