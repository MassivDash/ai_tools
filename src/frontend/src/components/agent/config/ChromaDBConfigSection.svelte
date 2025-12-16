<script lang="ts">
  import { onMount } from 'svelte'
  import SearchableList from '../../ui/SearchableList.svelte'
  import HelpIcon from '../../ui/HelpIcon.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { Collection, ModelInfo } from '../types'
  import type { ModelNote } from '../../modelNotes/types'

  export let chromadbEnabled: boolean = false
  export let collections: Collection[] = []
  export let models: ModelInfo[] = []
  export let selectedCollection: string = ''
  export let selectedEmbeddingModel: string = ''
  export let loadingCollections: boolean = false
  export let loadingModels: boolean = false
  export let onToggle: () => void
  export let onCollectionSelect: (collection: Collection) => void
  export let onModelSelect: (model: ModelInfo) => void

  let modelNotes: Map<string, ModelNote> = new Map()

  const loadModelNotes = async () => {
    try {
      const response = await axiosBackendInstance.get<{ notes: ModelNote[] }>(
        'model-notes'
      )
      const notesMap = new Map<string, ModelNote>()
      for (const note of response.data.notes) {
        if (note.platform === 'ollama' && note.model_name) {
          notesMap.set(note.model_name, note)
        }
      }
      modelNotes = notesMap
    } catch (err: any) {
      console.error('❌ Failed to load model notes:', err)
    }
  }

  onMount(() => {
    loadModelNotes().catch(console.error)
  })

  const getCollectionKey = (collection: Collection) => collection.id
  const getCollectionLabel = (collection: Collection) => collection.name
  const getCollectionSubtext = (collection: Collection) => {
    const parts = []
    if (collection.count !== undefined) {
      parts.push(`${collection.count} documents`)
    }
    return parts.join(' • ')
  }

  const getModelKey = (model: ModelInfo) => model.name
  const getModelLabel = (model: ModelInfo) => model.name
  const getModelSubtext = (model: ModelInfo) => {
    const parts = []
    if (model.size) {
      parts.push(model.size)
    }
    if (model.modified) {
      parts.push(model.modified)
    }
    return parts.join(' • ')
  }

  // Get model note for Ollama model (matched by name)
  const getModelNote = (model: ModelInfo): ModelNote | null => {
    return modelNotes.get(model.name) || null
  }

  const getModelFavorite = (model: ModelInfo): boolean => {
    const note = getModelNote(model)
    return note?.is_favorite || false
  }

  const getModelTags = (model: ModelInfo): string[] => {
    const note = getModelNote(model)
    return note?.tags || []
  }

  const getModelNotes = (model: ModelInfo): string => {
    const note = getModelNote(model)
    if (!note?.notes) return ''
    // Return a preview (first 100 chars)
    return note.notes.length > 100
      ? note.notes.substring(0, 100) + '...'
      : note.notes
  }
</script>

<div class="config-section">
  <div class="section-label">ChromaDB Knowledge Base:</div>
  <label class="tool-checkbox">
    <input
      type="checkbox"
      checked={chromadbEnabled}
      onchange={onToggle}
      class="checkbox-input"
    />
    <span>Enable ChromaDB</span>
    <HelpIcon
      text="Enable ChromaDB to allow the agent to search your knowledge base collections for relevant information."
    />
  </label>

  {#if chromadbEnabled}
    <!-- Collection Selection -->
    <div class="config-subsection">
      <div class="label-with-help">
        <label for="collection" class="custom-label">Collection</label>
        <HelpIcon
          text="Select the ChromaDB collection to use for searches. The agent will query this collection when it needs information."
        />
      </div>
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

    <!-- Embedding Model Selection -->
    <div class="config-subsection">
      <div class="label-with-help">
        <label for="embedding-model" class="custom-label">Embedding Model</label
        >
        <HelpIcon
          text="The Ollama model used to generate embeddings for query searches. Must match the model used when uploading documents."
        />
      </div>
      {#if loadingModels}
        <div class="loading">Loading models...</div>
      {:else if models.length > 0}
        <SearchableList
          items={models}
          searchPlaceholder="Search models..."
          emptyMessage="No models found"
          getItemKey={getModelKey}
          getItemLabel={getModelLabel}
          getItemSubtext={getModelSubtext}
          getItemFavorite={getModelFavorite}
          getItemTags={getModelTags}
          getItemNotes={getModelNotes}
          selectedKey={selectedEmbeddingModel || null}
          onselect={onModelSelect}
        />
      {:else}
        <div class="no-items">
          <p>No Ollama models found</p>
          <p class="hint-small">
            Run 'ollama pull &lt;model&gt;' to download models
          </p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .config-section {
    margin-bottom: 2rem;
  }

  .config-subsection {
    margin-bottom: 1.5rem;
    margin-top: 1rem;
    margin-left: 1.5rem;
  }

  .section-label {
    display: block;
    margin-bottom: 0.75rem;
    font-weight: 600;
    color: var(--text-primary, #333);
    font-size: 1rem;
    transition: color 0.3s ease;
  }

  .tool-checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-weight: 600;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .checkbox-input {
    width: 1.25rem;
    height: 1.25rem;
    cursor: pointer;
    accent-color: var(--accent-color, #2196f3);
  }

  .label-with-help {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    position: relative;
  }

  .custom-label {
    font-weight: 600;
    color: var(--text-primary, #333);
    font-size: 1rem;
    transition: color 0.3s ease;
  }

  .loading {
    padding: 1rem;
    text-align: center;
    color: var(--text-secondary, #666);
    transition: color 0.3s ease;
  }

  .no-items {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary, #666);
    transition: color 0.3s ease;
  }

  .no-items .hint-small {
    font-size: 0.85rem;
    color: var(--text-tertiary, #999);
    margin-top: 0.5rem;
    transition: color 0.3s ease;
  }
</style>
