<script lang="ts">
  import Input from '../ui/Input.svelte'
  import Button from '../ui/Button.svelte'
  import SearchableList from '../ui/SearchableList.svelte'
  import Accordion from '../ui/Accordion.svelte'
  import HelpIcon from '../ui/HelpIcon.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import {
    LlamaConfigRequestSchema,
    buildLlamaConfigPayload
  } from '../../validation/llamaConfig.ts'
  import type { ModelNote } from '../modelNotes/types'

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
    threads?: number | null
    threads_batch?: number | null
    predict?: number | null
    batch_size?: number | null
    ubatch_size?: number | null
    flash_attn?: boolean | null
    mlock?: boolean | null
    no_mmap?: boolean | null
    gpu_layers?: number | null
    model?: string | null
  }

  interface LlamaServerResponse {
    success: boolean
    message: string
  }

  let localModels: ModelInfo[] = []
  let modelNotes: Map<string, ModelNote> = new Map()
  let config: ConfigResponse = { hf_model: '', ctx_size: 10240 }
  let newHfModel = '' // Display value (filename)
  let newHfModelBackend = '' // Backend value (path or hf_format)
  let newCtxSize = 10240
  // Advanced options
  let newThreads: number | '' = ''
  let newThreadsBatch: number | '' = ''
  let newPredict: number | '' = ''
  let newBatchSize: number | '' = ''
  let newUbatchSize: number | '' = ''
  let newFlashAttn: boolean = false
  let newMlock: boolean = false
  let newNoMmap: boolean = false
  let newGpuLayers: number | '' = ''
  let newModel: string = ''
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

  // Extract filename from path, or return as-is if not a path
  // Determine if a string looks like a local file path
  const isLocalPath = (str: string): boolean => {
    if (!str) return false
    // Check for absolute paths or typical relative path starts
    // On Windows checking for \ or Drive: is safer
    return (
      str.startsWith('/') ||
      str.startsWith('./') ||
      str.startsWith('../') ||
      str.includes('\\') ||
      /^[a-zA-Z]:\\/.test(str)
    )
  }

  // Extract filename from path for display, but keep HF IDs (user/repo) as is
  const getDisplayValue = (pathOrName: string): string => {
    if (!pathOrName) return ''
    // Only strip path if it actually looks like a filesystem path
    if (isLocalPath(pathOrName)) {
      // Extract just the filename
      const parts = pathOrName.split(/[/\\]/)
      return parts[parts.length - 1] || pathOrName
    }
    return pathOrName
  }

  const loadConfig = async () => {
    try {
      const response = await axiosBackendInstance.get<ConfigResponse>(
        'llama-server/config'
      )
      config = response.data
      // Store backend value (full path or hf_format)
      newHfModelBackend = config.hf_model
      // Extract filename only if it's a loal path; preserve HF format
      newHfModel = getDisplayValue(config.hf_model)
      newCtxSize = config.ctx_size
      newThreads = config.threads ?? ''
      newThreadsBatch = config.threads_batch ?? ''
      newPredict = config.predict ?? ''
      newBatchSize = config.batch_size ?? ''
      newUbatchSize = config.ubatch_size ?? ''
      newFlashAttn = config.flash_attn ?? false
      newMlock = config.mlock ?? false
      newNoMmap = config.no_mmap ?? false
      newGpuLayers = config.gpu_layers ?? ''
      newModel = config.model ?? ''
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
        if (note.platform === 'llama') {
          // Create key from model_name or model_path
          const key = note.model_name || note.model_path || ''
          if (key) {
            notesMap.set(key, note)
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

  // Sync backend value when user types manually (if not matching a model from list)
  $: {
    // If newHfModel doesn't match any model name, assume it's a manual entry
    const matchingModel = localModels.find((m) => m.name === newHfModel)
    if (
      !matchingModel &&
      newHfModel &&
      newHfModel !== getDisplayValue(newHfModelBackend)
    ) {
      // User typed something manually, use it as backend value
      newHfModelBackend = newHfModel
      // Clear the specific model path to avoid conflicts with -hf flag
      newModel = ''
    }
  }

  const handleModelSelect = (model: ModelInfo) => {
    // Determine the best display value
    // Prioritize HF format if available, then fallback to name (filename)
    newHfModel = model.hf_format || model.name
    // Store the backend value (hf_format preferred, fallback to path)
    newHfModelBackend = model.hf_format || model.path || model.name
    // Also set the model path if available for the --model flag
    if (model.path) {
      newModel = model.path
    }
  }

  const handleSave = async () => {
    savingConfig = true
    error = ''

    // Validate required field (UI already enforces this, but add safety check)
    if (!newHfModel.trim()) {
      error = 'HuggingFace model is required'
      savingConfig = false
      return
    }

    try {
      // Build payload using helper function
      // Use backend value if available, otherwise use display value
      const hfModelValue = newHfModelBackend || newHfModel
      const payload = buildLlamaConfigPayload({
        hf_model: hfModelValue,
        ctx_size: newCtxSize,
        threads: newThreads,
        threads_batch: newThreadsBatch,
        predict: newPredict,
        batch_size: newBatchSize,
        ubatch_size: newUbatchSize,
        flash_attn: newFlashAttn,
        mlock: newMlock,
        no_mmap: newNoMmap,
        gpu_layers: newGpuLayers,
        model: newModel
      })

      // Validate with Zod
      const validationResult = LlamaConfigRequestSchema.safeParse(payload)

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        savingConfig = false
        return
      }

      const response = await axiosBackendInstance.post<LlamaServerResponse>(
        'llama-server/config',
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

  // Find matching note for a model
  const getModelNote = (model: ModelInfo): ModelNote | null => {
    // Try matching by name first
    let note = modelNotes.get(model.name)
    if (note) return note

    // Try matching by path
    if (model.path) {
      note = modelNotes.get(model.path)
      if (note) return note
    }

    // Try matching by hf_format
    if (model.hf_format) {
      note = modelNotes.get(model.hf_format)
      if (note) return note
    }

    return null
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
    <div style="display: flex; align-items: center; gap: 0.75rem;">
      <MaterialIcon name="server-network" width="28" height="28" />
      <h4>Server Configuration</h4>
    </div>
    <button class="close-button" on:click={onClose} aria-label="Close">
      ✕
    </button>
  </div>
  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    <!-- Model Name First -->
    <div class="config-section">
      <div class="label-with-help">
        <label for="hf-model" class="custom-label">HuggingFace Model</label>
        <HelpIcon
          text="Enter a HuggingFace model identifier. llama.cpp will download it if needed."
        />
      </div>
      <Input
        id="hf-model"
        label=""
        type="text"
        bind:value={newHfModel}
        placeholder="e.g., unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL"
      />
    </div>

    <!-- Model List -->
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
          getItemFavorite={getModelFavorite}
          getItemTags={getModelTags}
          getItemNotes={getModelNotes}
          selectedKey={(() => {
            // Match by name (filename) or by backend value (path/hf_format)
            const selected = localModels.find(
              (m) =>
                m.name === newHfModel ||
                m.path === newHfModelBackend ||
                m.hf_format === newHfModelBackend ||
                m.path === newHfModelBackend ||
                m.hf_format === newHfModelBackend ||
                getDisplayValue(m.path) === newHfModel ||
                getDisplayValue(newHfModelBackend) === m.name
            )
            if (!selected) return null
            // Use path as key (should be unique), fallback to name
            return selected.path || selected.name || selected.hf_format || null
          })()}
          onselect={handleModelSelect}
        />
      {:else}
        <div class="no-models">
          <p>No GGUF models found in ~/.cache/llama.cpp/</p>
          <p class="hint-small">Models will appear here once downloaded</p>
        </div>
      {/if}
    </div>

    <!-- Basic Options -->
    <div class="config-section">
      <div class="label-with-help">
        <label for="ctx-size" class="custom-label">Context Size</label>
        <HelpIcon
          text="Size of the prompt context. In other words, the amount of tokens that the LLM can remember at once. Increasing the context size also increases the memory requirements for the LLM. Every model has a context size limit, when this argument is set to 0, llama.cpp tries to use it."
        />
      </div>
      <Input
        id="ctx-size"
        label=""
        type="number"
        bind:value={newCtxSize}
        min="1"
      />
    </div>

    <!-- Advanced Options Accordion -->
    <Accordion title="Advanced Options">
      <div class="advanced-options">
        <div class="config-section">
          <div class="label-with-help">
            <label for="threads" class="custom-label">Threads</label>
            <HelpIcon
              text="Amount of CPU threads used by LLM. Default value is -1, which tells llama.cpp to detect the amount of cores in the system. This behavior is probably good enough for most of people, so unless you have exotic hardware setup and you know what you're doing - leave it on default."
            />
          </div>
          <Input
            id="threads"
            label=""
            type="number"
            bind:value={newThreads}
            placeholder="-1 (auto-detect)"
          />
        </div>

        <div class="config-section">
          <div class="label-with-help">
            <label for="threads-batch" class="custom-label">Threads Batch</label
            >
            <HelpIcon
              text="Amount of CPU threads used for batch processing. Default value is -1, which tells llama.cpp to detect the amount of cores in the system."
            />
          </div>
          <Input
            id="threads-batch"
            label=""
            type="number"
            bind:value={newThreadsBatch}
            placeholder="-1 (auto-detect)"
          />
        </div>

        <div class="config-section">
          <div class="label-with-help">
            <label for="predict" class="custom-label">Predict (N Predict)</label
            >
            <HelpIcon
              text="Number of tokens to predict. When LLM generates text, it stops either after generating end-of-message token (when it decides that the generated sentence is over), or after hitting this limit. Default is -1, which makes the LLM generate text ad infinitum. If we want to limit it to context size, we can set it to -2."
            />
          </div>
          <Input
            id="predict"
            label=""
            type="number"
            bind:value={newPredict}
            placeholder="-1 (unlimited)"
          />
        </div>

        <div class="config-section">
          <div class="label-with-help">
            <label for="batch-size" class="custom-label">Batch Size</label>
            <HelpIcon
              text="Amount of tokens fed to the LLM in single processing step. Optimal value of this argument depends on your hardware, model, and context size - i encourage experimentation, but defaults are probably good enough for start."
            />
          </div>
          <Input
            id="batch-size"
            label=""
            type="number"
            bind:value={newBatchSize}
            min="1"
          />
        </div>

        <div class="config-section">
          <div class="label-with-help">
            <label for="ubatch-size" class="custom-label">UBatch Size</label>
            <HelpIcon
              text="Amount of tokens fed to the LLM in single processing step (unified batch). Optimal value of this argument depends on your hardware, model, and context size - i encourage experimentation, but defaults are probably good enough for start."
            />
          </div>
          <Input
            id="ubatch-size"
            label=""
            type="number"
            bind:value={newUbatchSize}
            min="1"
          />
        </div>

        <div class="checkbox-with-help">
          <div class="checkbox-wrapper">
            <label class="checkbox-label">
              <input
                type="checkbox"
                bind:checked={newFlashAttn}
                class="checkbox-input"
              />
              <span>Flash Attention</span>
            </label>
            <HelpIcon
              text="Flash attention is an optimization that's supported by most recent models. Enabling it should improve the generation performance for some models. llama.cpp will simply throw a warning when a model that doesn't support flash attention is loaded, so i keep it on at all times without any issues."
            />
          </div>
        </div>

        <div class="checkbox-with-help">
          <div class="checkbox-wrapper">
            <label class="checkbox-label">
              <input
                type="checkbox"
                bind:checked={newMlock}
                class="checkbox-input"
              />
              <span>MLock</span>
            </label>
            <HelpIcon
              text="This option is called exactly like Linux function that it uses underneath. On Windows, it uses VirtualLock. If you have enough virtual memory (RAM or VRAM) to load the whole model into, you can use this parameter to prevent OS from swapping it to the hard drive. Enabling it can increase the performance of text generation, but may slow everything else down in return if you hit the virtual memory limit of your machine."
            />
          </div>
        </div>

        <div class="checkbox-with-help">
          <div class="checkbox-wrapper">
            <label class="checkbox-label">
              <input
                type="checkbox"
                bind:checked={newNoMmap}
                class="checkbox-input"
              />
              <span>No MMAP</span>
            </label>
            <HelpIcon
              text="By default, llama.cpp will map the model to memory (using mmap on Linux and CreateFileMappingA on Windows). Using this switch will disable this behavior."
            />
          </div>
        </div>

        <div class="config-section">
          <div class="label-with-help">
            <label for="gpu-layers" class="custom-label">GPU Layers</label>
            <HelpIcon
              text="If GPU offloading is available, this parameter will set the maximum amount of LLM layers to offload to GPU. Number and size of layers is dependent on the used model. Usually, if we want to load the whole model to GPU, we can set this parameter to some unreasonably large number like 999. For partial offloading, you must experiment yourself. llama.cpp must be built with GPU support, otherwise this option will have no effect. If you have multiple GPUs, you may also want to look at --split-mode and --main-gpu arguments."
            />
          </div>
          <Input
            id="gpu-layers"
            label=""
            type="number"
            bind:value={newGpuLayers}
            min="0"
          />
        </div>

        <div class="config-section">
          <div class="label-with-help">
            <label for="model" class="custom-label">Model Path</label>
            <HelpIcon
              text="Path to the GGUF model file. This is an alternative to using HuggingFace model identifier."
            />
          </div>
          <Input
            id="model"
            label=""
            type="text"
            bind:value={newModel}
            placeholder="Path to GGUF model file"
          />
        </div>
      </div>
    </Accordion>
  </div>
  <div class="config-footer">
    <Button variant="secondary" onclick={onClose}>Cancel</Button>
    <Button
      variant="primary"
      onclick={handleSave}
      disabled={savingConfig || !newHfModel.trim()}
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
    border-radius: 8px;
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
    border-top-left-radius: 8px;
    border-bottom-left-radius: 8px;
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
    border-radius: 8px;
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

  .advanced-options {
    display: flex;
    flex-direction: column;
  }

  .advanced-options .config-section {
    margin-bottom: 1.5rem;
  }

  .checkbox-with-help {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    position: relative;
  }

  .checkbox-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .checkbox-label {
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

  @media screen and (max-width: 768px) {
    .config-panel {
      width: 100%;
      min-width: 100%;
      max-width: 100%;
    }
  }
</style>
