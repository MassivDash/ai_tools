<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { onMount, onDestroy } from 'svelte'
  import Terminal from './terminal.svelte'

  interface LlamaServerStatus {
    active: boolean
    port: number
  }

  interface LlamaServerResponse {
    success: boolean
    message: string
  }

  interface ModelInfo {
    name: string
    path: string
    size?: number
    hf_format?: string
  }

  interface ModelsResponse {
    local_models: ModelInfo[]
  }

  interface ConfigResponse {
    hf_model: string
    ctx_size: number
  }

  let serverStatus: LlamaServerStatus = { active: false, port: 8080 }
  let loading = false
  let error = ''
  let statusWs: WebSocket | null = null
  let showConfig = false
  let showTerminal = false
  let isStarting = false
  let localModels: ModelInfo[] = []
  let config: ConfigResponse = { hf_model: '', ctx_size: 10240 }
  let newHfModel = ''
  let newCtxSize = 10240
  let loadingModels = false
  let savingConfig = false
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null

  type StatusWebSocketMessage = {
    type: 'status'
    active: boolean
    port: number
  }

  const getWebSocketUrl = (): string => {
    const baseUrl = import.meta.env.PUBLIC_API_URL || window.location.origin
    const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws'
    const wsBase = baseUrl.replace(/^https?:\/\//, '').replace(/\/$/, '')
    return `${wsProtocol}://${wsBase}/api/llama-server/status/ws`
  }

  const connectStatusWebSocket = () => {
    try {
      const wsUrl = getWebSocketUrl()
      console.log('🔌 Connecting to status WebSocket:', wsUrl)
      statusWs = new WebSocket(wsUrl)

      statusWs.onopen = () => {
        console.log('✅ Status WebSocket connected')
        error = ''
      }

      statusWs.onmessage = (event) => {
        try {
          const message: StatusWebSocketMessage = JSON.parse(event.data)

          if (message.type === 'status') {
            const wasActive = serverStatus.active
            serverStatus = { active: message.active, port: message.port }
            console.log('🔄 Server status updated:', serverStatus)

            // If server just became active, stop the starting flag
            if (serverStatus.active && !wasActive && isStarting) {
              isStarting = false
            }
          }
        } catch (err) {
          console.error('❌ Failed to parse status WebSocket message:', err)
        }
      }

      statusWs.onerror = (err) => {
        console.error('❌ Status WebSocket error:', err)
        error = 'WebSocket connection error'
      }

      statusWs.onclose = () => {
        console.log('🔌 Status WebSocket closed, reconnecting...')
        statusWs = null
        // Reconnect after 2 seconds
        if (reconnectTimeout) {
          clearTimeout(reconnectTimeout)
        }
        reconnectTimeout = setTimeout(() => {
          connectStatusWebSocket()
        }, 2000)
      }
    } catch (err: any) {
      console.error('❌ Failed to connect status WebSocket:', err)
      error = err.message || 'Failed to connect to status stream'
    }
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
      const response = await axiosBackendInstance.get<ModelsResponse>(
        'llama-server/models'
      )
      localModels = response.data.local_models
      console.log('📦 Loaded models:', localModels)
    } catch (err: any) {
      console.error('❌ Failed to load models:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load models'
    } finally {
      loadingModels = false
    }
  }

  const openConfig = async () => {
    showConfig = true
    await loadConfig()
    await loadModels()
  }

  const closeConfig = () => {
    showConfig = false
    error = ''
  }

  const saveConfig = async () => {
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
        closeConfig()
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

  const selectLocalModel = (model: ModelInfo) => {
    // Use HF format if available, otherwise use the path
    newHfModel = model.hf_format || model.path
  }

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

  const startServer = async () => {
    loading = true
    error = ''
    isStarting = true
    showTerminal = true
    try {
      console.log('🚀 Starting llama server...')
      const response =
        await axiosBackendInstance.post<LlamaServerResponse>(
          'llama-server/start'
        )
      console.log('✅ Start response:', response.data)
      if (response.data.success) {
        // Status will be updated via WebSocket
        isStarting = true
      } else {
        error = response.data.message
        isStarting = false
      }
    } catch (err: any) {
      console.error('❌ Failed to start server:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to start server'
      isStarting = false
    } finally {
      loading = false
    }
  }

  const stopServer = async () => {
    loading = true
    error = ''
    isStarting = false
    try {
      console.log('🛑 Stopping llama server...')
      const response =
        await axiosBackendInstance.post<LlamaServerResponse>(
          'llama-server/stop'
        )
      console.log('✅ Stop response:', response.data)
      if (response.data.success) {
        serverStatus.active = false
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('❌ Failed to stop server:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to stop server'
    } finally {
      loading = false
    }
  }

  onMount(() => {
    // Connect to status WebSocket
    connectStatusWebSocket()
  })

  onDestroy(() => {
    if (statusWs) {
      statusWs.close()
      statusWs = null
    }
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
    }
  })
</script>

<div class="ai-chat">
  <div class="chat-header">
    <h3>Llama.cpp Server</h3>
    <div class="header-actions">
      <button
        onclick={openConfig}
        class="config-button"
        title="Configure server"
      >
        ⚙️ Config
      </button>
      <button
        onclick={() => (showTerminal = !showTerminal)}
        class="terminal-toggle-button"
        title="Toggle terminal"
      >
        {showTerminal ? '📺 Hide Terminal' : '💻 Show Terminal'}
      </button>
      {#if serverStatus.active}
        <button
          onclick={stopServer}
          disabled={loading}
          class="stop-button"
          title="Stop server"
        >
          {loading ? 'Stopping...' : 'Stop Server'}
        </button>
      {:else}
        <button
          onclick={startServer}
          disabled={loading}
          class="start-button"
          title="Start server"
        >
          {loading ? 'Starting...' : 'Start Server'}
        </button>
      {/if}
    </div>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="content-area" class:has-terminal={showTerminal}>
    <div class="terminal-sidebar" class:visible={showTerminal}>
      <Terminal />
    </div>
    <div class="main-content" class:with-terminal={showTerminal}>
      {#if serverStatus.active}
        <div class="iframe-container">
          <iframe
            src="http://localhost:8080"
            class="llama-iframe"
            title="Llama.cpp WebUI"
          ></iframe>
        </div>
      {:else}
        <div class="empty-state">
          <p>🦙 Llama.cpp Server is not running</p>
          <p class="hint">
            Click "Start Server" to launch the llama.cpp server and access the
            web UI
          </p>
          <p class="hint-small">Server will be available at localhost:8080</p>
        </div>
      {/if}
    </div>
  </div>
</div>

{#if showConfig}
  <div
    class="modal-overlay"
    role="button"
    tabindex="0"
    onclick={closeConfig}
    onkeydown={(e) => e.key === 'Escape' && closeConfig()}
  >
    <div
      class="modal-content"
      role="dialog"
      aria-labelledby="config-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <h3 id="config-title">Server Configuration</h3>
        <button class="close-button" onclick={closeConfig} aria-label="Close"
          >×</button
        >
      </div>

      <div class="modal-body">
        {#if error}
          <div class="error">{error}</div>
        {/if}

        <div class="config-section">
          <label for="hf-model">HuggingFace Model:</label>
          <input
            id="hf-model"
            type="text"
            bind:value={newHfModel}
            placeholder="e.g., unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL"
            class="config-input"
          />
          <p class="config-hint">
            Enter a HuggingFace model identifier. llama.cpp will download it if
            needed.
          </p>
        </div>

        <div class="config-section">
          <label for="ctx-size">Context Size:</label>
          <input
            id="ctx-size"
            type="number"
            bind:value={newCtxSize}
            min="1"
            class="config-input"
          />
          <p class="config-hint">Maximum context window size for the model.</p>
        </div>

        <div class="config-section">
          <div class="section-label">Local GGUF Models:</div>
          {#if loadingModels}
            <div class="loading-models">Loading models...</div>
          {:else if localModels.length > 0}
            <div class="models-list" role="list">
              {#each localModels as model}
                <button
                  type="button"
                  class="model-item"
                  onclick={() => selectLocalModel(model)}
                  class:selected={newHfModel ===
                    (model.hf_format || model.path)}
                >
                  <div class="model-name">{model.name}</div>
                  <div class="model-details">
                    {#if model.hf_format}
                      <span class="model-hf-format" title="HuggingFace format"
                        >{model.hf_format}</span
                      >
                    {:else}
                      <span class="model-path" title="File path"
                        >{model.path}</span
                      >
                    {/if}
                    {#if model.size}
                      <span class="model-size"
                        >{formatFileSize(model.size)}</span
                      >
                    {/if}
                  </div>
                </button>
              {/each}
            </div>
          {:else}
            <div class="no-models">
              <p>No GGUF models found in ~/.cache/llama.cpp/</p>
              <p class="hint-small">Models will appear here once downloaded</p>
            </div>
          {/if}
        </div>
      </div>

      <div class="modal-footer">
        <button onclick={closeConfig} class="cancel-button">Cancel</button>
        <button
          onclick={saveConfig}
          disabled={savingConfig || !newHfModel.trim()}
          class="save-button"
        >
          {savingConfig ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .ai-chat {
    width: 100%;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    min-height: 80vh;
    background-color: #fff;
  }

  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 2px solid #f0f0f0;
  }

  .chat-header h3 {
    margin: 0;
    color: #100f0f;
    font-size: 1.5rem;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .start-button,
  .stop-button,
  .config-button {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
    font-weight: 600;
  }

  .config-button,
  .terminal-toggle-button {
    background-color: #2196f3;
    color: white;
  }

  .config-button:hover:not(:disabled),
  .terminal-toggle-button:hover:not(:disabled) {
    background-color: #1976d2;
  }

  .terminal-toggle-button {
    font-size: 0.85rem;
  }

  .start-button {
    background-color: #4caf50;
    color: white;
  }

  .start-button:hover:not(:disabled) {
    background-color: #45a049;
  }

  .stop-button {
    background-color: #f44336;
    color: white;
  }

  .stop-button:hover:not(:disabled) {
    background-color: #da190b;
  }

  .start-button:disabled,
  .stop-button:disabled,
  .config-button:disabled {
    background-color: #ccc;
    cursor: not-allowed;
    opacity: 0.6;
  }

  .error {
    padding: 0.75rem;
    margin: 0 1rem;
    background-color: #fee;
    border: 1px solid #fcc;
    border-radius: 4px;
    color: #c33;
    font-size: 0.9rem;
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: row;
    min-height: 80vh;
    position: relative;
    overflow: hidden;
  }

  .terminal-sidebar {
    width: 70%;
    height: 100%;
    border-right: 1px solid #ddd;
    background-color: #1e1e1e;
    transform: translateX(-100%);
    transition: transform 0.3s ease-in-out;
    z-index: 10;
    display: flex;
    flex-direction: column;
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
  }

  .terminal-sidebar.visible {
    transform: translateX(0);
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    transition: margin-left 0.3s ease-in-out;
    margin-left: 0;
    min-width: 0;
    width: 100%;
  }

  .main-content.with-terminal {
    margin-left: 70%;
  }

  .iframe-container {
    flex: 1;
    width: 100%;
    min-height: 80vh;
    overflow: hidden;
    border: none;
  }

  .llama-iframe {
    width: 100%;
    height: 100%;
    min-height: 80vh;
    border: none;
    display: block;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #666;
    text-align: center;
    min-height: 80vh;
  }

  .empty-state p {
    margin: 0.5rem 0;
  }

  .empty-state .hint {
    font-size: 0.9rem;
    color: #999;
  }

  .empty-state .hint-small {
    font-size: 0.8rem;
    color: #aaa;
  }

  /* Modal Styles */
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 1rem;
  }

  .modal-content {
    background-color: white;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    max-width: 700px;
    width: 100%;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 1px solid #ddd;
  }

  .modal-header h3 {
    margin: 0;
    color: #100f0f;
    font-size: 1.25rem;
  }

  .close-button {
    background: none;
    border: none;
    font-size: 2rem;
    cursor: pointer;
    color: #666;
    line-height: 1;
    padding: 0;
    width: 2rem;
    height: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-button:hover {
    color: #333;
  }

  .modal-body {
    padding: 1.5rem;
    overflow-y: auto;
    flex: 1;
  }

  .config-section {
    margin-bottom: 2rem;
  }

  .config-section label,
  .config-section .section-label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: #333;
  }

  .config-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 1rem;
    font-family: inherit;
    box-sizing: border-box;
  }

  .config-input:focus {
    outline: none;
    border-color: #2196f3;
  }

  .config-hint {
    margin-top: 0.5rem;
    font-size: 0.85rem;
    color: #666;
  }

  .loading-models {
    padding: 1rem;
    text-align: center;
    color: #666;
  }

  .models-list {
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid #ddd;
    border-radius: 4px;
  }

  .model-item {
    width: 100%;
    padding: 1rem;
    border: none;
    border-bottom: 1px solid #f0f0f0;
    background: white;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .model-item:hover {
    background-color: #f5f5f5;
  }

  .model-item.selected {
    background-color: #e3f2fd;
    border-left: 3px solid #2196f3;
  }

  .model-item:last-child {
    border-bottom: none;
  }

  .model-name {
    font-weight: 600;
    color: #333;
    margin-bottom: 0.25rem;
  }

  .model-details {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.85rem;
    color: #666;
  }

  .model-path,
  .model-hf-format {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-right: 1rem;
  }

  .model-hf-format {
    color: #2196f3;
    font-weight: 500;
  }

  .model-size {
    color: #999;
    white-space: nowrap;
  }

  .no-models {
    padding: 2rem;
    text-align: center;
    color: #666;
  }

  .no-models .hint-small {
    font-size: 0.85rem;
    color: #999;
    margin-top: 0.5rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1.5rem;
    border-top: 1px solid #ddd;
  }

  .cancel-button,
  .save-button {
    padding: 0.75rem 1.5rem;
    border-radius: 4px;
    font-size: 1rem;
    cursor: pointer;
    border: none;
    font-weight: 600;
    transition: all 0.2s;
  }

  .cancel-button {
    background-color: #f5f5f5;
    color: #666;
  }

  .cancel-button:hover {
    background-color: #e8e8e8;
  }

  .save-button {
    background-color: #2196f3;
    color: white;
  }

  .save-button:hover:not(:disabled) {
    background-color: #1976d2;
  }

  .save-button:disabled {
    background-color: #ccc;
    cursor: not-allowed;
    opacity: 0.6;
  }

  @media screen and (max-width: 768px) {
    .modal-content {
      max-width: 100%;
      max-height: 95vh;
    }

    .ai-chat {
      min-height: 70vh;
    }

    .iframe-container {
      min-height: 70vh;
    }

    .llama-iframe {
      min-height: 70vh;
    }

    .terminal-sidebar {
      width: 100%;
      min-width: 100%;
      max-width: 100%;
    }

    .main-content.with-terminal {
      margin-left: 0;
    }
  }
</style>
