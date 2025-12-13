<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ChromaDBResponse, ChromaDBCollection, CreateCollectionRequest } from '../../types/chromadb.ts'
  import Button from '../ui/Button.svelte'
  import Input from '../ui/Input.svelte'

  const dispatch = createEventDispatcher()

  let showForm = false
  let collectionName = ''
  let metadata: Record<string, string> = {}
  let loading = false
  let error = ''

  const toggleForm = () => {
    showForm = !showForm
    if (!showForm) {
      // Reset form when closing
      collectionName = ''
      metadata = {}
      error = ''
    }
  }

  const addMetadataField = () => {
    const key = `key_${Object.keys(metadata).length + 1}`
    metadata[key] = ''
  }

  const removeMetadataField = (key: string) => {
    delete metadata[key]
    metadata = { ...metadata } // Trigger reactivity
  }

  const createCollection = async () => {
    if (!collectionName.trim()) {
      error = 'Collection name is required'
      return
    }

    loading = true
    error = ''

    try {
      console.log('📝 Creating collection:', collectionName)
      const request: CreateCollectionRequest = {
        name: collectionName.trim(),
        metadata: Object.keys(metadata).length > 0 ? metadata : undefined
      }

      const response = await axiosBackendInstance.post<ChromaDBResponse<ChromaDBCollection>>(
        'chromadb/collections',
        request
      )

      if (response.data.success && response.data.data) {
        console.log('✅ Collection created:', response.data.data)
        dispatch('created', response.data.data)
        // Reset form
        collectionName = ''
        metadata = {}
        showForm = false
      } else {
        error = response.data.error || 'Failed to create collection'
      }
    } catch (err: any) {
      console.error('❌ Error creating collection:', err)
      error = err.response?.data?.error || err.message || 'Failed to create collection'
    } finally {
      loading = false
    }
  }
</script>

<div class="create-collection">
  <Button onclick={toggleForm}>
    {showForm ? '❌ Cancel' : '➕ Create Collection'}
  </Button>

  {#if showForm}
    <div class="form-container">
      <h3>Create New Collection</h3>

      {#if error}
        <div class="error-message">❌ {error}</div>
      {/if}

      <div class="form-group">
        <label for="collection-name">Collection Name *</label>
        <Input
          id="collection-name"
          bind:value={collectionName}
          placeholder="Enter collection name..."
          disabled={loading}
        />
      </div>

      <div class="form-group">
        <div class="metadata-header">
          <label>Metadata (Optional)</label>
          <Button onclick={addMetadataField} type="button" variant="secondary">
            ➕ Add Field
          </Button>
        </div>

        {#if Object.keys(metadata).length > 0}
          <div class="metadata-fields">
            {#each Object.entries(metadata) as [key, value]}
              <div class="metadata-field">
                <Input
                  placeholder="Key"
                  value={key}
                  oninput={(e) => {
                    const newKey = e.target.value
                    if (newKey !== key) {
                      metadata[newKey] = metadata[key]
                      delete metadata[key]
                      metadata = { ...metadata }
                    }
                  }}
                />
                <Input
                  placeholder="Value"
                  value={value}
                  oninput={(e) => {
                    metadata[key] = e.target.value
                    metadata = { ...metadata }
                  }}
                />
                <button
                  class="remove-btn"
                  onclick={() => removeMetadataField(key)}
                  type="button"
                >
                  🗑️
                </button>
              </div>
            {/each}
          </div>
        {:else}
          <p class="hint">No metadata fields. Click "Add Field" to add some.</p>
        {/if}
      </div>

      <div class="form-actions">
        <Button onclick={createCollection} disabled={loading || !collectionName.trim()}>
          {loading ? 'Creating...' : '✅ Create Collection'}
        </Button>
        <Button onclick={toggleForm} variant="secondary" disabled={loading}>
          Cancel
        </Button>
      </div>
    </div>
  {/if}
</div>

<style>
  .create-collection {
    margin-bottom: 2rem;
  }

  .form-container {
    margin-top: 1rem;
    padding: 1.5rem;
    background: var(--bg-primary, white);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    box-shadow: 0 2px 8px var(--shadow, rgba(0, 0, 0, 0.1));
  }

  .form-container h3 {
    margin: 0 0 1.5rem 0;
    color: var(--text-primary, #100f0f);
  }

  .error-message {
    padding: 0.75rem;
    background: #fee;
    border: 1px solid #fcc;
    border-radius: 4px;
    color: #c33;
    margin-bottom: 1rem;
  }

  .form-group {
    margin-bottom: 1.5rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #100f0f);
  }

  .metadata-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .metadata-fields {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .metadata-field {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .metadata-field :global(input) {
    flex: 1;
  }

  .remove-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    padding: 0.5rem;
    opacity: 0.6;
    transition: opacity 0.2s;
  }

  .remove-btn:hover {
    opacity: 1;
  }

  .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary, #999);
    font-style: italic;
  }

  .form-actions {
    display: flex;
    gap: 1rem;
    margin-top: 1.5rem;
  }
</style>


