<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type {
    ChromaDBResponse,
    ChromaDBCollection,
    CreateCollectionRequest
  } from '../../types/chromadb.ts'
  import Button from '../ui/Button.svelte'
  import IconButton from '../ui/IconButton.svelte'
  import PlusIcon from '../ui/icons/PlusIcon.svelte'
  import XIcon from '../ui/icons/XIcon.svelte'
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

      const response = await axiosBackendInstance.post<
        ChromaDBResponse<ChromaDBCollection>
      >('chromadb/collections', request)

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
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to create collection'
    } finally {
      loading = false
    }
  }
</script>

<div class="create-collection">
  <IconButton
    variant="info"
    onclick={toggleForm}
    title={showForm ? 'Cancel' : 'Create Collection'}
  >
    {#if showForm}
      <XIcon width="24" height="24" />
    {:else}
      <PlusIcon width="24" height="24" />
    {/if}
  </IconButton>

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
          <span class="metadata-label">Metadata (Optional)</span>
          <Button onclick={addMetadataField} type="button" variant="secondary">
            <PlusIcon width="24" height="24" /> Add Field
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
                  {value}
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
        <Button
          onclick={createCollection}
          disabled={loading || !collectionName.trim()}
          variant="success"
        >
          {loading ? 'Creating...' : 'Create Collection'}
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
    position: relative;
  }

  .form-container {
    position: absolute;
    top: calc(100% + 0.5rem);
    right: 0;
    padding: 1.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    box-shadow: 0 4px 12px var(--shadow);
    z-index: 10;
    min-width: 400px;
    max-width: 500px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .form-container h3 {
    margin: 0 0 1.5rem 0;
    color: var(--text-primary, #100f0f);
  }

  .error-message {
    padding: 0.75rem;
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

  .form-group {
    margin-bottom: 1.5rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .metadata-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .metadata-label {
    font-weight: 600;
    color: var(--text-primary);
    transition: color 0.3s ease;
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
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    cursor: pointer;
    font-size: 1.2rem;
    padding: 0.5rem;
    opacity: 0.8;
    transition:
      opacity 0.2s,
      background-color 0.3s ease,
      border-color 0.3s ease;
    color: var(--text-primary);
  }

  .remove-btn:hover {
    opacity: 1;
    background: var(--bg-tertiary);
    border-color: var(--border-color-hover);
  }

  .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary);
    font-style: italic;
    transition: color 0.3s ease;
  }

  .form-actions {
    display: flex;
    gap: 1rem;
    margin-top: 1.5rem;
  }
</style>
