<script lang="ts">
  import Button from '../ui/Button.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type {
    AgentConfig,
    AgentConfigResponse,
    Collection,
    ModelInfo,
    ChromaDBResponse
  } from './types'
  import ChromaDBConfigSection from './config/ChromaDBConfigSection.svelte'
  import ToolsConfigSection from './config/ToolsConfigSection.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import CheckboxWithHelp from '../ui/CheckboxWithHelp.svelte'

  export let isOpen: boolean = false
  export let onClose: () => void
  export let onSave: () => void

  let collections: Collection[] = []
  let models: ModelInfo[] = []
  let enabledTools: string[] = []
  let chromadbEnabled = false
  let selectedCollection = ''
  let selectedEmbeddingModel = ''
  let loadingCollections = false
  let loadingModels = false
  let savingConfig = false
  let error = ''

  const loadConfig = async () => {
    try {
      const response =
        await axiosBackendInstance.get<AgentConfig>('agent/config')
      // Backend returns enabled_tools as string[] (ToolType enum serialized to snake_case)
      enabledTools = response.data.enabled_tools || []

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
      console.error('Failed to load agent config:', err)
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
      console.error('Failed to load collections:', err)
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
      console.error('Failed to load models:', err)
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
      // Ensure enabled_tools are in the correct format (snake_case matching ToolType enum)
      // Backend expects: ['financial_data', 'website_check'] etc.
      const payload = {
        enabled_tools: enabledTools, // Already in correct format from tool.tool_type
        chromadb: chromadbEnabled
          ? {
              collection: selectedCollection,
              embedding_model: selectedEmbeddingModel
            }
          : undefined
      }

      const response = await axiosBackendInstance.post<AgentConfigResponse>(
        'agent/config',
        payload
      )

      if (response.data.success) {
        await loadConfig()
        // Store update will be handled by parent component via onSave callback
        onSave()
        onClose()
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('Failed to save agent config:', err)
      error =
        err.response?.data?.error ||
        err.response?.data?.message ||
        err.message ||
        'Failed to save agent config'
    } finally {
      savingConfig = false
    }
  }
</script>

<div class="config-panel" class:visible={isOpen}>
  <div class="config-header">
    <div style="display: flex; align-items: center; gap: 0.75rem;">
      <MaterialIcon name="robot-confused" width="28" height="28" />
      <h4>Agent Configuration</h4>
    </div>
    <button class="close-button" onclick={onClose} aria-label="Close">
      ✕
    </button>
  </div>
  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    <ChromaDBConfigSection
      {chromadbEnabled}
      {collections}
      {models}
      {selectedCollection}
      {selectedEmbeddingModel}
      {loadingCollections}
      {loadingModels}
      onToggle={handleChromaDBToggle}
      onCollectionSelect={handleCollectionSelect}
      onModelSelect={handleModelSelect}
    />

    <ToolsConfigSection {enabledTools} onToggle={handleToolToggle} />
  </div>
  <div class="config-footer">
    <Button variant="secondary" onclick={onClose}>Cancel</Button>
    <Button
      variant="primary"
      onclick={handleSave}
      disabled={savingConfig ||
        (chromadbEnabled && (!selectedCollection || !selectedEmbeddingModel))}
    >
      {savingConfig ? 'Saving...' : 'Save'}
    </Button>
  </div>
</div>

<style>
  .config-panel {
    width: 60%;
    height: 100%;
    background-color: var(--bg-primary, #fff);
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
    max-height: 100vh;
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
    border-top-left-radius: 8px;
    border-bottom-left-radius: 8px;
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
    border-radius: 8px;
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
    border-radius: 8px;
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

  .error {
    padding: 0.75rem;
    margin-bottom: 1rem;
    background-color: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
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

  .section-container {
    margin-bottom: 1.5rem;
    background-color: var(--bg-primary, #fff);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    overflow: hidden;
  }

  .section-header {
    padding: 0.75rem 1rem;
    background-color: var(--bg-secondary, #f9f9f9);
    border-bottom: 1px solid var(--border-color, #e0e0e0);
  }

  .section-header h5 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary, #100f0f);
  }

  .section-content {
    padding: 1rem;
  }
</style>
