<script lang="ts">
  import Input from '../ui/Input.svelte'
  import Button from '../ui/Button.svelte'
  import SearchableList from '../ui/SearchableList.svelte'
  import HelpIcon from '../ui/HelpIcon.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { onMount } from 'svelte'

  export let isOpen: boolean = false
  export let onClose: () => void
  export let onSave: () => void

  interface Collection {
    id: string
    name: string
    metadata?: Record<string, string>
    count?: number
  }

  interface ModelInfo {
    name: string
    size?: string
    modified?: string
  }

  interface AgentConfig {
    enabled_tools: string[]
    chromadb?: {
      collection: string
      embedding_model: string
    }
  }

  interface AgentConfigResponse {
    success: boolean
    message: string
  }

  interface ChromaDBResponse<T> {
    success: boolean
    data?: T
    error?: string
  }

  let collections: Collection[] = []
  let models: ModelInfo[] = []
  let enabledTools: string[] = []
  let chromadbEnabled = false
  let selectedCollection = ''
  let selectedEmbeddingModel = ''
  let financialDataEnabled = false
  let loadingCollections = false
  let loadingModels = false
  let savingConfig = false
  let error = ''

  const loadConfig = async () => {
    try {
      const response = await axiosBackendInstance.get<AgentConfig>('agent/config')
      enabledTools = response.data.enabled_tools || []
      financialDataEnabled = enabledTools.includes('financial_data')
      
      // ChromaDB is now separate from enabled_tools
      if (response.data.chromadb) {
        chromadbEnabled = true
        selectedCollection = response.data.chromadb.collection
        selectedEmbeddingModel = response.data.chromadb.embedding_model
      } else {
        chromadbEnabled = false
        selectedCollection = ''
        selectedEmbeddingModel = ''
      }
    } catch (err: any) {
      console.error('❌ Failed to load agent config:', err)
    }
  }

  const loadCollections = async () => {
    loadingCollections = true
    try {
      const response = await axiosBackendInstance.get<
        ChromaDBResponse<Collection[]>
      >('chromadb/collections')
      if (response.data.success && response.data.data) {
        collections = response.data.data
      } else {
        error = response.data.error || 'Failed to load collections'
      }
    } catch (err: any) {
      console.error('❌ Failed to load collections:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load collections'
    } finally {
      loadingCollections = false
    }
  }

  const loadModels = async () => {
    loadingModels = true
    try {
      const response = await axiosBackendInstance.get<{
        models: ModelInfo[]
      }>('chromadb/models')
      models = response.data.models
    } catch (err: any) {
      console.error('❌ Failed to load models:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load models'
    } finally {
      loadingModels = false
    }
  }

  $: if (isOpen) {
    loadConfig().catch(console.error)
    loadCollections().catch(console.error)
    loadModels().catch(console.error)
  }

  const handleChromaDBToggle = () => {
    chromadbEnabled = !chromadbEnabled
    if (!chromadbEnabled) {
      selectedCollection = ''
      selectedEmbeddingModel = ''
    }
  }

  const handleToolToggle = (tool: string) => {
    if (enabledTools.includes(tool)) {
      enabledTools = enabledTools.filter((t) => t !== tool)
    } else {
      enabledTools = [...enabledTools, tool]
    }
    
    if (tool === 'financial_data') {
      financialDataEnabled = enabledTools.includes('financial_data')
    }
  }

  const handleCollectionSelect = (collection: Collection) => {
    selectedCollection = collection.name
  }

  const handleModelSelect = (model: ModelInfo) => {
    selectedEmbeddingModel = model.name
  }

  const handleSave = async () => {
    savingConfig = true
    error = ''

    // Validate ChromaDB config if enabled
    if (chromadbEnabled) {
      if (!selectedCollection.trim()) {
        error = 'Please select a ChromaDB collection'
        savingConfig = false
        return
      }
      if (!selectedEmbeddingModel.trim()) {
        error = 'Please select an embedding model'
        savingConfig = false
        return
      }
    }

    try {
      const payload = {
        enabled_tools: enabledTools,
        chromadb: chromadbEnabled
          ? {
              collection: selectedCollection,
              embedding_model: selectedEmbeddingModel,
            }
          : undefined,
      }

      const response = await axiosBackendInstance.post<AgentConfigResponse>(
        'agent/config',
        payload
      )

      if (response.data.success) {
        await loadConfig()
        onSave()
        onClose()
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('❌ Failed to save agent config:', err)
      error =
        err.response?.data?.error ||
        err.response?.data?.message ||
        err.message ||
        'Failed to save agent config'
    } finally {
      savingConfig = false
    }
  }

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
</script>

<div class="config-panel" class:visible={isOpen}>
  <div class="config-header">
    <h4>Agent Configuration</h4>
    <button class="close-button" onclick={onClose} aria-label="Close">
      ✕
    </button>
  </div>
  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    <!-- ChromaDB Section -->
    <div class="config-section">
      <div class="section-label">ChromaDB Knowledge Base:</div>
      <label class="tool-checkbox">
        <input
          type="checkbox"
          checked={chromadbEnabled}
          onchange={handleChromaDBToggle}
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
              onselect={handleCollectionSelect}
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
            <label for="embedding-model" class="custom-label"
              >Embedding Model</label
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
              selectedKey={selectedEmbeddingModel || null}
              onselect={handleModelSelect}
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

    <!-- Tools Section -->
    <div class="config-section">
      <div class="section-label">Tools:</div>
      <div class="tools-list">
        <label class="tool-checkbox">
          <input
            type="checkbox"
            checked={financialDataEnabled}
            onchange={() => handleToolToggle('financial_data')}
            class="checkbox-input"
          />
          <span>My Financial Data</span>
          <HelpIcon
            text="Enable this tool to allow the agent to access your financial data including recent purchases and transactions."
          />
        </label>
      </div>
    </div>
  </div>
  <div class="config-footer">
    <Button variant="secondary" onclick={onClose}>Cancel</Button>
    <Button
      variant="primary"
      onclick={handleSave}
      disabled={savingConfig || (chromadbEnabled && (!selectedCollection || !selectedEmbeddingModel))}
    >
      {savingConfig ? 'Saving...' : 'Save'}
    </Button>
  </div>
</div>

<style>
  .config-panel {
    width: 70%;
    height: 100%;
    background-color: var(--bg-primary, #fff);
    border-left: 1px solid var(--border-color, #ddd);
    transform: translateX(100%);
    transition:
      transform 0.3s ease-in-out,
      background-color 0.3s ease,
      border-color 0.3s ease;
    z-index: 10;
    display: flex;
    flex-direction: column;
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    box-shadow: -2px 0 8px var(--shadow, rgba(0, 0, 0, 0.1));
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

  .tools-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
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

  @media screen and (max-width: 768px) {
    .config-panel {
      width: 100%;
      min-width: 100%;
      max-width: 100%;
    }
  }
</style>
