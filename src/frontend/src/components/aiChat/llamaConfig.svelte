<script lang="ts">
  import Input from '../ui/Input.svelte'
  import Button from '../ui/Button.svelte'
  import SearchableList from '../ui/SearchableList.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

  export let isOpen: boolean = false
  export let onClose: () => void
  export let onSave: () => void

  interface ModelInfo {
    name: string
    path: string
    size?: number
    hf_format?: string
  }

  interface ConfigResponse {
    hf_model: string
    ctx_size: number
  }

  interface LlamaServerResponse {
    success: boolean
    message: string
  }

  let localModels: ModelInfo[] = []
  let config: ConfigResponse = { hf_model: '', ctx_size: 10240 }
  let newHfModel = ''
  let newCtxSize = 10240
  let loadingModels = false
  let savingConfig = false
  let error = ''

  const formatFileSize = (bytes?: number): string => {
    if (!bytes) return 'Unknown size'
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    let size = bytes
    let unitIndex = 0
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024
      unitIndex++
    }
    return `${size.toFixed(2)} ${units[unitIndex]}`
  }

  const loadConfig = async () => {
    try {
      const response = await axiosBackendInstance.get<ConfigResponse>(
        'llama-server/config'
      )
      config = response.data
      newHfModel = config.hf_model
      newCtxSize = config.ctx_size
      console.log('📋 Config loaded:', config)
    } catch (err: any) {
      console.error('❌ Failed to load config:', err)
    }
  }

  const loadModels = async () => {
    loadingModels = true
    try {
      const response = await axiosBackendInstance.get<{
        local_models: ModelInfo[]
      }>('llama-server/models')
      localModels = response.data.local_models
      console.log('📦 Loaded models:', localModels)
    } catch (err: any) {
      console.error('❌ Failed to load models:', err)
      error = err.response?.data?.error || err.message || 'Failed to load models'
    } finally {
      loadingModels = false
    }
  }

  $: if (isOpen) {
    // Load config and models when modal opens
    loadConfig().catch(console.error)
    loadModels().catch(console.error)
  }

  const handleModelSelect = (model: ModelInfo) => {
    newHfModel = model.hf_format || model.path
  }

  const handleSave = async () => {
    savingConfig = true
    error = ''
    try {
      const response = await axiosBackendInstance.post<LlamaServerResponse>(
        'llama-server/config',
        {
          hf_model: newHfModel.trim() || undefined,
          ctx_size: newCtxSize > 0 ? newCtxSize : undefined
        }
      )
      console.log('✅ Config saved:', response.data)
      if (response.data.success) {
        await loadConfig()
        onSave()
        onClose()
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('❌ Failed to save config:', err)
      error = err.response?.data?.error || err.message || 'Failed to save config'
    } finally {
      savingConfig = false
    }
  }

  // Use a combination to ensure uniqueness - path should be unique, but add name as fallback
  const getModelKey = (model: ModelInfo, index: number) => {
    // Use path as primary key (should be unique), fallback to name + index if path is missing
    if (model.path) return model.path
    if (model.name) return `${model.name}-${index}`
    if (model.hf_format) return `${model.hf_format}-${index}`
    return `model-${index}`
  }
  const getModelLabel = (model: ModelInfo) => model.name
  const getModelSubtext = (model: ModelInfo) => {
    const parts = []
    if (model.hf_format) {
      parts.push(model.hf_format)
    } else if (model.path) {
      parts.push(model.path)
    }
    if (model.size) {
      parts.push(formatFileSize(model.size))
    }
    return parts.join(' • ')
  }
</script>

<div class="config-panel" class:visible={isOpen}>
  <div class="config-header">
    <h4>Server Configuration</h4>
    <button class="close-button" on:click={onClose} aria-label="Close">
      ✕
    </button>
  </div>
  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    <div class="config-section">
      <Input
        id="hf-model"
        label="HuggingFace Model"
        type="text"
        bind:value={newHfModel}
        placeholder="e.g., unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL"
        hint="Enter a HuggingFace model identifier. llama.cpp will download it if needed."
      />
    </div>

    <div class="config-section">
      <Input
        id="ctx-size"
        label="Context Size"
        type="number"
        bind:value={newCtxSize}
        min="1"
        hint="Maximum context window size for the model."
      />
    </div>

    <div class="config-section">
      <div class="section-label">Local GGUF Models:</div>
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
          selectedKey={(() => {
            const selected = localModels.find(m => (m.hf_format || m.path) === newHfModel)
            if (!selected) return null
            // Use path as key (should be unique), fallback to name
            return selected.path || selected.name || selected.hf_format || null
          })()}
          on:select={(e) => handleModelSelect(e.detail)}
        />
      {:else}
        <div class="no-models">
          <p>No GGUF models found in ~/.cache/llama.cpp/</p>
          <p class="hint-small">Models will appear here once downloaded</p>
        </div>
      {/if}
    </div>
  </div>
  <div class="config-footer">
    <Button variant="secondary" on:click={onClose}>Cancel</Button>
    <Button
      variant="primary"
      on:click={handleSave}
      disabled={savingConfig || !newHfModel.trim()}
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
    transition: transform 0.3s ease-in-out, background-color 0.3s ease, border-color 0.3s ease;
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
    transition: border-color 0.3s ease, background-color 0.3s ease;
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
    transition: background-color 0.2s, color 0.3s ease;
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
    transition: border-color 0.3s ease, background-color 0.3s ease;
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
    transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease;
  }

  @media screen and (max-width: 768px) {
    .config-panel {
      width: 100%;
      min-width: 100%;
      max-width: 100%;
    }
  }
</style>

