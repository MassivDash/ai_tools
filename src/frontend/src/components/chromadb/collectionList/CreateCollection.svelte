<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type {
    ChromaDBResponse,
    ChromaDBCollection,
    CreateCollectionRequest
  } from '@types'
  import { collections, selectedCollection } from '@stores/chromadb.ts'
  import { CreateCollectionRequestSchema } from '@validation/chromadb.ts'
  import Button from '@ui/Button.svelte'
  import IconButton from '@ui/IconButton.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import Input from '@ui/Input.svelte'

  let showForm = false
  let collectionName = ''
  let metadata: Record<string, string> = {}
  let distanceMetric: 'cosine' | 'l2' | 'ip' | undefined = 'cosine'
  let loading = false
  let error = ''

  const toggleForm = () => {
    showForm = !showForm
    if (!showForm) {
      // Reset form when closing
      collectionName = ''
      metadata = {}
      distanceMetric = 'cosine'
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
    loading = true
    error = ''

    try {
      // Validate with Zod
      const validationResult = CreateCollectionRequestSchema.safeParse({
        name: collectionName.trim(),
        metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
        distance_metric: distanceMetric
      })

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        loading = false
        return
      }

      const request: CreateCollectionRequest = validationResult.data

      const response = await axiosBackendInstance.post<
        ChromaDBResponse<ChromaDBCollection>
      >('chromadb/collections', request)

      if (response.data.success && response.data.data) {
        // Add to collections store
        collections.update((cols) => [...cols, response.data.data!])
        // Select the newly created collection
        selectedCollection.set(response.data.data)
        // Reset form
        collectionName = ''
        metadata = {}
        distanceMetric = 'cosine'
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
      <MaterialIcon name="close" width="24" height="24" />
    {:else}
      <MaterialIcon name="plus" width="24" height="24" />
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
        <label for="distance-metric">Distance Metric *</label>
        <select
          id="distance-metric"
          bind:value={distanceMetric}
          disabled={loading}
          class="select-input"
        >
          <option value="cosine"
            >Cosine (Recommended for semantic search)</option
          >
          <option value="l2">L2 / Euclidean</option>
          <option value="ip">Inner Product</option>
        </select>
        <p class="hint">
          Cosine is recommended for semantic search with normalized embeddings
          (like nomic-embed-text).
        </p>
      </div>

      <div class="form-group">
        <div class="metadata-header">
          <span class="metadata-label">Metadata (Optional)</span>
          <Button onclick={addMetadataField} type="button" variant="secondary">
            <MaterialIcon name="plus" width="24" height="24" /> Add Field
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
                  title="Remove field"
                >
                  <MaterialIcon name="close" width="18" height="18" />
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
    max-height: 400px;
    overflow-y: auto;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  /* Custom scrollbar styling - Webkit browsers (Chrome, Safari, Edge) */
  .form-container::-webkit-scrollbar {
    width: 10px;
  }

  .form-container::-webkit-scrollbar-track {
    background: var(--bg-secondary, rgba(0, 0, 0, 0.05));
    border-radius: 8px;
    margin: 4px 0;
  }

  .form-container::-webkit-scrollbar-thumb {
    background: var(--border-color, rgba(0, 0, 0, 0.3));
    border-radius: 8px;
    border: 2px solid var(--bg-primary);
    transition: background-color 0.2s ease;
  }

  .form-container::-webkit-scrollbar-thumb:hover {
    background: var(--border-color-hover, rgba(0, 0, 0, 0.5));
  }

  /* Custom scrollbar styling - Firefox */
  .form-container {
    scrollbar-width: thin;
    scrollbar-color: var(--border-color, rgba(0, 0, 0, 0.3))
      var(--bg-secondary, rgba(0, 0, 0, 0.05));
  }

  /* Dark theme scrollbar adjustments */
  @media (prefers-color-scheme: dark) {
    .form-container::-webkit-scrollbar-track {
      background: var(--bg-secondary, rgba(255, 255, 255, 0.05));
    }

    .form-container::-webkit-scrollbar-thumb {
      background: var(--border-color, rgba(255, 255, 255, 0.3));
    }

    .form-container::-webkit-scrollbar-thumb:hover {
      background: var(--border-color-hover, rgba(255, 255, 255, 0.5));
    }

    .form-container {
      scrollbar-color: var(--border-color, rgba(255, 255, 255, 0.3))
        var(--bg-secondary, rgba(255, 255, 255, 0.05));
    }
  }

  .form-container h3 {
    margin: 0 0 1.5rem 0;
    color: var(--text-primary, #100f0f);
  }

  .error-message {
    padding: 0.75rem;
    background: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
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
    border-radius: 8px;
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

  .remove-btn:hover {
    opacity: 1;
    background: var(--bg-tertiary);
    border-color: var(--border-color-hover);
    color: var(--accent-color, #c33);
  }

  .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary);
    font-style: italic;
    transition: color 0.3s ease;
    margin-top: 0.25rem;
  }

  .select-input {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 1rem;
    cursor: pointer;
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .select-input:hover:not(:disabled) {
    border-color: var(--border-color-hover);
  }

  .select-input:focus {
    outline: none;
    border-color: var(--accent-color, #007bff);
    box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
  }

  .select-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .form-actions {
    display: flex;
    gap: 1rem;
    margin-top: 1.5rem;
  }
</style>
