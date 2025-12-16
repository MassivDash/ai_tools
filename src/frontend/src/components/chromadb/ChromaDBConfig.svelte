<script lang="ts">
  import Input from '../ui/Input.svelte'
  import Button from '../ui/Button.svelte'
  import SearchableList from '../ui/SearchableList.svelte'
  import HelpIcon from '../ui/HelpIcon.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import {
    ChromaDBConfigRequestSchema,
    buildChromaDBConfigPayload
  } from '@validation/chromadbConfig.ts'
  import type { ModelNote } from '../modelNotes/types'

  export let isOpen: boolean = false
  export let onClose: () => void
  export let onSave: () => void

  interface ModelInfo {
    name: string
    size?: string
    modified?: string
  }

  interface ConfigResponse {
    embedding_model: string
    query_model: string
  }

  interface ConfigUpdateResponse {
    success: boolean
    message: string
  }

  let localModels: ModelInfo[] = []
  let modelNotes: Map<string, ModelNote> = new Map()
  let config: ConfigResponse = {
    embedding_model: 'nomic-embed-text',
    query_model: 'nomic-embed-text'
  }
  let newEmbeddingModel = 'nomic-embed-text'
  // query_model is no longer used - queries always use embedding_model
  // Keeping variable for compatibility but not displaying in UI
  let loadingModels = false
  let savingConfig = false
  let error = ''

  const loadConfig = async () => {
    try {
      const response =
        await axiosBackendInstance.get<ConfigResponse>('chromadb/config')
      config = response.data
      newEmbeddingModel = config.embedding_model
      // query_model is no longer used in UI, but we keep it for API compatibility
    } catch (err: any) {
      console.error('❌ Failed to load config:', err)
    }
  }

  const loadModels = async () => {
    loadingModels = true
    try {
      const response = await axiosBackendInstance.get<{
        models: ModelInfo[]
      }>('chromadb/models')
      localModels = response.data.models
    } catch (err: any) {
      console.error('❌ Failed to load models:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load models'
    } finally {
      loadingModels = false
    }
  }

  const loadModelNotes = async () => {
    try {
      const response = await axiosBackendInstance.get<{ notes: ModelNote[] }>(
        'model-notes'
      )
      const notesMap = new Map<string, ModelNote>()
      for (const note of response.data.notes) {
        if (note.platform === 'ollama') {
          // Use model_name as key for Ollama models
          if (note.model_name) {
            notesMap.set(note.model_name, note)
          }
        }
      }
      modelNotes = notesMap
    } catch (err: any) {
      console.error('❌ Failed to load model notes:', err)
    }
  }

  $: if (isOpen) {
    // Load config and models when modal opens
    loadConfig().catch(console.error)
    loadModels().catch(console.error)
    loadModelNotes().catch(console.error)
  }

  const handleEmbeddingModelSelect = (model: ModelInfo) => {
    newEmbeddingModel = model.name
  }

  const handleSave = async () => {
    savingConfig = true
    error = ''

    // Validate required field
    if (!newEmbeddingModel.trim()) {
      error = 'Embedding model is required'
      savingConfig = false
      return
    }

    try {
      // Build payload using helper function
      // Note: query_model is no longer used (queries always use embedding_model)
      // but we send it as undefined to let backend handle defaults
      const payload = buildChromaDBConfigPayload({
        embedding_model: newEmbeddingModel,
        query_model: undefined // Queries now always use embedding_model
      })

      // Validate with Zod
      const validationResult = ChromaDBConfigRequestSchema.safeParse(payload)

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        savingConfig = false
        return
      }

      const response = await axiosBackendInstance.post<ConfigUpdateResponse>(
        'chromadb/config',
        validationResult.data
      )
      if (response.data.success) {
        await loadConfig()
        onSave()
        onClose()
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('❌ Failed to save config:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to save config'
    } finally {
      savingConfig = false
    }
  }

  // Use model name as key (should be unique)
  const getModelKey = (model: ModelInfo, index: number) => {
    if (model.name) return model.name
    return `model-${index}`
  }
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

<div class="config-panel" class:visible={isOpen}>
  <div class="config-header">
    <h4>ChromaDB Configuration</h4>
    <button class="close-button" on:click={onClose} aria-label="Close">
      ✕
    </button>
  </div>
  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    <!-- Embedding Model -->
    <div class="config-section">
      <div class="label-with-help">
        <label for="embedding-model" class="custom-label">Embedding Model</label
        >
        <HelpIcon
          text="The Ollama model used to generate embeddings for documents when uploading to ChromaDB collections."
        />
      </div>
      <Input
        id="embedding-model"
        label=""
        type="text"
        bind:value={newEmbeddingModel}
        placeholder="e.g., nomic-embed-text"
      />
    </div>

    <!-- Embedding Model List -->
    <div class="config-section">
      <div class="section-label">Available Ollama Models:</div>
      {#if loadingModels}
        <div class="loading-models">Loading models...</div>
      {:else if localModels.length > 0}
        <SearchableList
          items={localModels}
          searchPlaceholder="Search models..."
          emptyMessage="No models found"
          getItemKey={getModelKey}
          getItemLabel={getModelLabel}
          getItemSubtext={getModelSubtext}
          getItemFavorite={getModelFavorite}
          getItemTags={getModelTags}
          getItemNotes={getModelNotes}
          selectedKey={(() => {
            const selected = localModels.find(
              (m) => m.name === newEmbeddingModel
            )
            if (!selected) return null
            return selected.name
          })()}
          onselect={handleEmbeddingModelSelect}
        />
      {:else}
        <div class="no-models">
          <p>No Ollama models found</p>
          <p class="hint-small">
            Run 'ollama pull &lt;model&gt;' to download models
          </p>
        </div>
      {/if}
    </div>
  </div>
  <div class="config-footer">
    <Button variant="secondary" onclick={onClose}>Cancel</Button>
    <Button
      variant="primary"
      onclick={handleSave}
      disabled={savingConfig || !newEmbeddingModel.trim()}
    >
      {savingConfig ? 'Saving...' : 'Save'}
    </Button>
  </div>
</div>

<style>
  .config-panel {
    width: 70vw;
    max-width: 980px;
    height: calc(100vh - 80px);
    background-color: var(--bg-primary, #fff);
    border-left: 1px solid var(--border-color, #ddd);
    transform: translateX(100%);
    transition:
      transform 0.3s ease-in-out,
      background-color 0.3s ease,
      border-color 0.3s ease;
    z-index: 1002;
    display: flex;
    flex-direction: column;
    position: fixed;
    right: 0;
    top: 80px;
    bottom: 0;
    box-shadow: -2px 0 8px var(--shadow, rgba(0, 0, 0, 0.1));
    overflow-y: auto;
  }

  .config-panel.visible {
    transform: translateX(0);
  }

  .config-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease;
  }

  .config-header h4 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    font-size: 1.2rem;
    font-weight: 600;
    transition: color 0.3s ease;
  }

  .close-button {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--text-secondary, #666);
    padding: 0;
    width: 2rem;
    height: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition:
      background-color 0.2s,
      color 0.3s ease;
  }

  .close-button:hover {
    background-color: var(--bg-tertiary, #e0e0e0);
    color: var(--text-primary, #100f0f);
  }

  .config-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    min-height: 0;
  }

  .config-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease;
  }

  .config-section {
    margin-bottom: 2rem;
  }

  .section-label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .loading-models {
    padding: 1rem;
    text-align: center;
    color: var(--text-secondary, #666);
    transition: color 0.3s ease;
  }

  .no-models {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary, #666);
    transition: color 0.3s ease;
  }

  .no-models .hint-small {
    font-size: 0.85rem;
    color: var(--text-tertiary, #999);
    margin-top: 0.5rem;
    transition: color 0.3s ease;
  }

  .error {
    padding: 0.75rem;
    margin-bottom: 1rem;
    background-color: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 4px;
    color: var(--accent-color, #c33);
    font-size: 0.9rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
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

  @media screen and (max-width: 768px) {
    .config-panel {
      width: 100vw;
      min-width: 100vw;
      max-width: 100vw;
      top: 80px;
      height: calc(100vh - 80px);
    }
  }
</style>
