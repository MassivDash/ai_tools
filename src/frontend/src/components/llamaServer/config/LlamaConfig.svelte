<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import Button from '../../ui/Button.svelte'
  import IconButton from '../../ui/IconButton.svelte'
  import Accordion from '../../ui/Accordion.svelte'

  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import {
    LlamaConfigRequestSchema,
    buildLlamaConfigPayload
  } from '@validation/llamaConfig.ts'
  import type { ModelNote } from '@types'
  import type { AgentConfig } from '../../agent/types'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'
  import CheckboxWithHelp from '../../ui/CheckboxWithHelp.svelte'
  import ModelSelector from './ModelSelector.svelte'
  import { getDisplayValue } from './utils'

  export let isOpen: boolean = false
  export let onClose: () => void
  export let onSave: (() => void) | undefined = undefined

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
  let debugLogging = false // Agent debug logging state

  const loadConfig = async () => {
    try {
      const response = await axiosBackendInstance.get<ConfigResponse>(
        'llama-server/config'
      )
      config = response.data
      newHfModelBackend = config.hf_model
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
      newModel = config.model ?? ''
    } catch (err: any) {
      console.error('Failed to load config:', err)
    }

    // Load agent config for debug logging
    try {
      const response =
        await axiosBackendInstance.get<AgentConfig>('agent/config')
      debugLogging = !!response.data.debug_logging
    } catch (err: any) {
      console.error('Failed to load agent config:', err)
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
      console.error('Failed to load models:', err)
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
          const key = note.model_name || note.model_path || ''
          if (key) {
            notesMap.set(key, note)
          }
        }
      }
      modelNotes = notesMap
    } catch (err: any) {
      console.error('Failed to load model notes:', err)
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
      newHfModelBackend = newHfModel
      newModel = ''
    }
  }

  const handleModelSelect = (model: ModelInfo) => {
    newHfModel = model.hf_format || model.name
    newHfModelBackend = model.hf_format || model.path || model.name
    if (model.path) {
      newModel = model.path
    }
  }

  const handleSave = async () => {
    savingConfig = true
    error = ''

    if (!newHfModel.trim()) {
      error = 'HuggingFace model is required'
      savingConfig = false
      return
    }

    try {
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
        // Also save agent config for debug logging
        try {
          await axiosBackendInstance.post('agent/config', {
            debug_logging: debugLogging
          })
        } catch (agentErr) {
          console.error(
            'Failed to save agent config (debug logging):',
            agentErr
          )
          // We don't block the main save on this failure, but maybe show a warning?
        }

        await loadConfig()
        if (onSave) onSave()
        onClose()
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('Failed to save config:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to save config'
    } finally {
      savingConfig = false
    }
  }
</script>

<div class="config-panel" class:visible={isOpen}>
  <div class="config-header">
    <div style="display: flex; align-items: center; gap: 0.75rem;">
      <MaterialIcon name="server-network" width="28" height="28" />
      <h4>Server Configuration</h4>
    </div>
    <IconButton
      variant="ghost"
      onclick={onClose}
      title="Close"
      aria-label="Close"
      iconSize={24}
    >
      <MaterialIcon name="close" width="24" height="24" />
    </IconButton>
  </div>
  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    <!-- Model Name First -->
    <div class="config-section">
      <LabelWithHelp
        id="hf-model"
        label="HuggingFace Model"
        helpText="Enter a HuggingFace model identifier. llama.cpp will download it if needed."
      />
      <Input
        id="hf-model"
        label=""
        type="text"
        bind:value={newHfModel}
        placeholder="e.g., unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL"
      />
    </div>

    <!-- Model List -->
    <ModelSelector
      {loadingModels}
      {localModels}
      {modelNotes}
      {newHfModel}
      {newHfModelBackend}
      onSelect={handleModelSelect}
    />

    <!-- Basic Options -->
    <div class="config-section">
      <LabelWithHelp
        id="ctx-size"
        label="Context Size"
        helpText="Size of the prompt context. In other words, the amount of tokens that the LLM can remember at once. Increasing the context size also increases the memory requirements for the LLM. Every model has a context size limit, when this argument is set to 0, llama.cpp tries to use it."
      />
      <Input
        id="ctx-size"
        label=""
        type="number"
        bind:value={newCtxSize}
        min="1"
      />
    </div>

    <!-- Agent Debug Logging (from Agent Config) -->
    <div class="config-section">
      <CheckboxWithHelp
        bind:checked={debugLogging}
        label="Debug Conversation Logging"
        helpText="Writes detailed logs of the agent conversation (system prompts, thinking, tool calls, results) to a single file in public/logs per conversation."
      />
    </div>

    <!-- Advanced Options Accordion -->
    <Accordion title="Advanced Options">
      <div class="advanced-options">
        <div class="config-section">
          <LabelWithHelp
            id="threads"
            label="Threads"
            helpText="Amount of CPU threads used by LLM. Default value is -1, which tells llama.cpp to detect the amount of cores in the system. This behavior is probably good enough for most of people, so unless you have exotic hardware setup and you know what you're doing - leave it on default."
          />
          <Input
            id="threads"
            label=""
            type="number"
            bind:value={newThreads}
            placeholder="-1 (auto-detect)"
          />
        </div>

        <div class="config-section">
          <LabelWithHelp
            id="threads-batch"
            label="Threads Batch"
            helpText="Amount of CPU threads used for batch processing. Default value is -1, which tells llama.cpp to detect the amount of cores in the system."
          />
          <Input
            id="threads-batch"
            label=""
            type="number"
            bind:value={newThreadsBatch}
            placeholder="-1 (auto-detect)"
          />
        </div>

        <div class="config-section">
          <LabelWithHelp
            id="predict"
            label="Predict (N Predict)"
            helpText="Number of tokens to predict. When LLM generates text, it stops either after generating end-of-message token (when it decides that the generated sentence is over), or after hitting this limit. Default is -1, which makes the LLM generate text ad infinitum. If we want to limit it to context size, we can set it to -2."
          />
          <Input
            id="predict"
            label=""
            type="number"
            bind:value={newPredict}
            placeholder="-1 (unlimited)"
          />
        </div>

        <div class="config-section">
          <LabelWithHelp
            id="batch-size"
            label="Batch Size"
            helpText="Amount of tokens fed to the LLM in single processing step. Optimal value of this argument depends on your hardware, model, and context size - i encourage experimentation, but defaults are probably good enough for start."
          />
          <Input
            id="batch-size"
            label=""
            type="number"
            bind:value={newBatchSize}
            min="1"
          />
        </div>

        <div class="config-section">
          <LabelWithHelp
            id="ubatch-size"
            label="UBatch Size"
            helpText="Amount of tokens fed to the LLM in single processing step (unified batch). Optimal value of this argument depends on your hardware, model, and context size - i encourage experimentation, but defaults are probably good enough for start."
          />
          <Input
            id="ubatch-size"
            label=""
            type="number"
            bind:value={newUbatchSize}
            min="1"
          />
        </div>

        <CheckboxWithHelp
          bind:checked={newFlashAttn}
          label="Flash Attention"
          helpText="Flash attention is an optimization that's supported by most recent models. Enabling it should improve the generation performance for some models. llama.cpp will simply throw a warning when a model that doesn't support flash attention is loaded, so i keep it on at all times without any issues."
        />

        <CheckboxWithHelp
          bind:checked={newMlock}
          label="MLock"
          helpText="This option is called exactly like Linux function that it uses underneath. On Windows, it uses VirtualLock. If you have enough virtual memory (RAM or VRAM) to load the whole model into, you can use this parameter to prevent OS from swapping it to the hard drive. Enabling it can increase the performance of text generation, but may slow everything else down in return if you hit the virtual memory limit of your machine."
        />

        <CheckboxWithHelp
          bind:checked={newNoMmap}
          label="No MMAP"
          helpText="By default, llama.cpp will map the model to memory (using mmap on Linux and CreateFileMappingA on Windows). Using this switch will disable this behavior."
        />

        <div class="config-section">
          <LabelWithHelp
            id="gpu-layers"
            label="GPU Layers"
            helpText="If GPU offloading is available, this parameter will set the maximum amount of LLM layers to offload to GPU. Number and size of layers is dependent on the used model. Usually, if we want to load the whole model to GPU, we can set this parameter to some unreasonably large number like 999. For partial offloading, you must experiment yourself. llama.cpp must be built with GPU support, otherwise this option will have no effect. If you have multiple GPUs, you may also want to look at --split-mode and --main-gpu arguments."
          />
          <Input
            id="gpu-layers"
            label=""
            type="number"
            bind:value={newGpuLayers}
            min="0"
          />
        </div>

        <div class="config-section">
          <LabelWithHelp
            id="model"
            label="Model Path"
            helpText="Path to the GGUF model file. This is an alternative to using HuggingFace model identifier."
          />
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
</style>
