<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Terminal from '../llamaServer/terminal.svelte'
  import AgentConfig from './agentConfig.svelte'
  import LlamaConfig from '../llamaServer/llamaConfig.svelte'
  import ChatInterface from './chatInterface.svelte'
  import Button from '../ui/Button.svelte'
  import { useStatusWebSocket } from '../../hooks/useStatusWebSocket'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import { enabledTools as enabledToolsStore } from '../../stores/activeTools'

  interface LlamaServerStatus {
    active: boolean
    port: number
  }

  interface LlamaServerResponse {
    success: boolean
    message: string
  }

  interface AgentConfig {
    enabled_tools: string[]
    chromadb?: {
      collection: string
      embedding_model: string
    }
  }

  let serverStatus: LlamaServerStatus = { active: false, port: 8080 }
  let loading = false
  let error = ''
  let showConfig = false
  let showLlamaConfig = false
  let showTerminal = false
  let _isStarting = false

  // WebSocket hook for status updates
  const statusWs = useStatusWebSocket(
    (status) => {
      serverStatus = status
    },
    () => {
      // Server just became ready
      _isStarting = false
      showTerminal = false
    }
  )

  const startServer = async () => {
    // Check if server is already running
    if (serverStatus.active) {
      error = 'Server is already running'
      return
    }

    loading = true
    error = ''
    _isStarting = true
    showTerminal = true
    try {
      const response =
        await axiosBackendInstance.post<LlamaServerResponse>(
          'llama-server/start'
        )
      if (!response.data.success) {
        error = response.data.message
        _isStarting = false
      }
    } catch (err: any) {
      console.error('❌ Failed to start server:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to start server'
      _isStarting = false
    } finally {
      loading = false
    }
  }

  const stopServer = async () => {
    loading = true
    error = ''
    _isStarting = false
    try {
      const response =
        await axiosBackendInstance.post<LlamaServerResponse>(
          'llama-server/stop'
        )
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

  const loadAgentConfig = async () => {
    try {
      const response =
        await axiosBackendInstance.get<AgentConfig>('agent/config')
      const enabledToolsList = response.data.enabled_tools || []
      const chromadbEnabled = !!response.data.chromadb

      // Update enabled tools store with tools from config
      const toolsToAdd = new Set<string>()
      enabledToolsList.forEach((tool) => {
        toolsToAdd.add(tool)
      })
      if (chromadbEnabled) {
        toolsToAdd.add('chromadb')
      }
      enabledToolsStore.set(toolsToAdd)
    } catch (err: any) {
      console.error('❌ Failed to load agent config:', err)
    }
  }

  const handleConfigSave = () => {
    // Reload config after save to update badges
    loadAgentConfig()
  }

  const handleLlamaConfigSave = () => {
    // Llama config saved successfully
  }

  onMount(() => {
    statusWs.connect()
    // Load agent config on mount to show enabled tools as badges
    loadAgentConfig()
  })

  onDestroy(() => {
    statusWs.disconnect()
  })
</script>

<div class="ai-chat">
  <div class="chat-header">
    <h3>AI Agent</h3>
    <div class="header-actions">
      <Button
        variant="info"
        class="button-icon-only"
        onclick={() => {
          showConfig = !showConfig
          if (showConfig) showLlamaConfig = false
        }}
        title="Agent Config"
      >
        <MaterialIcon name="robot-confused" width="32" height="32" />
      </Button>
      <Button
        variant="info"
        class="button-icon-only"
        onclick={() => {
          showLlamaConfig = !showLlamaConfig
          if (showLlamaConfig) showConfig = false
        }}
        title="Llama Server Config"
      >
        <MaterialIcon name="server-network" width="32" height="32" />
      </Button>
      <Button
        variant="info"
        class="button-icon-only"
        onclick={() => (showTerminal = !showTerminal)}
        title={showTerminal ? 'Hide Terminal' : 'Show Terminal'}
      >
        <MaterialIcon name="console" width="32" height="32" />
      </Button>
      {#if serverStatus.active}
        <Button
          variant="danger"
          class="button-icon-only"
          onclick={stopServer}
          disabled={loading}
          title={loading ? 'Stopping...' : 'Stop Server'}
        >
          <MaterialIcon name="stop-circle" width="32" height="32" />
        </Button>
      {:else}
        <Button
          variant="success"
          class="button-icon-only"
          onclick={startServer}
          disabled={loading || serverStatus.active}
          title={loading
            ? 'Starting...'
            : serverStatus.active
              ? 'Server is running'
              : 'Start Server'}
        >
          <MaterialIcon name="play" width="32" height="32" />
        </Button>
      {/if}
    </div>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div
    class="content-area"
    class:has-terminal={showTerminal}
    class:has-config={showConfig}
    class:has-llama-config={showLlamaConfig}
  >
    <div class="terminal-sidebar" class:visible={showTerminal}>
      <Terminal />
    </div>
    <div
      class="main-content"
      class:with-terminal={showTerminal}
      class:with-config={showConfig}
      class:with-llama-config={showLlamaConfig}
    >
      {#if serverStatus.active}
        <ChatInterface />
      {:else}
        <div class="empty-state">
          <p>🤖 AI Agent is ready</p>
          <p class="hint">
            Click "Start Server" to launch the llama.cpp server and start
            chatting with the AI agent
          </p>
          <p class="hint-small">Server will be available at localhost:8080</p>
        </div>
      {/if}
    </div>
    <AgentConfig
      isOpen={showConfig}
      onClose={() => {
        showConfig = false
      }}
      onSave={handleConfigSave}
    />
    <LlamaConfig
      isOpen={showLlamaConfig}
      onClose={() => {
        showLlamaConfig = false
      }}
      onSave={handleLlamaConfigSave}
    />
  </div>
</div>

<style>
  .ai-chat {
    width: 100%;
    margin: 0;
    padding: 2rem;
    display: flex;
    flex-direction: column;
    min-height: 80vh;
    background-color: var(--bg-primary, #fff);
    transition: background-color 0.3s ease;
    box-sizing: border-box;
  }

  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;

    transition: border-color 0.3s ease;
  }

  .chat-header h3 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    font-size: 1.5rem;
    transition: color 0.3s ease;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .header-actions :global(.button-icon-only) {
    padding: 0.75rem !important;
    min-width: 3rem !important;
    min-height: 3rem !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .header-actions :global(.button-icon-only) :global(svg) {
    flex-shrink: 0;
  }

  .error {
    padding: 0.75rem;
    margin: 0 1rem;
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

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: row;
    min-height: 80vh;
    position: relative;
    overflow: hidden;
    width: 100%;
  }

  .terminal-sidebar {
    width: 70%;
    height: 100%;
    background-color: #1e1e1e;
    transform: translateX(-100%);
    transition:
      transform 0.3s ease-in-out,
      border-color 0.3s ease;
    z-index: 10;
    display: flex;
    flex-direction: column;
    position: absolute;
    left: -2px;
    top: 0;
    bottom: 0;
  }

  .terminal-sidebar.visible {
    transform: translateX(0);
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    transition:
      margin-left 0.3s ease-in-out,
      margin-right 0.3s ease-in-out;
    margin-left: 0;
    margin-right: 0;
    min-width: 0;
    width: 100%;
    background-color: var(--bg-primary, #fff);
    overflow: hidden;
    padding: 0;
  }

  .main-content.with-terminal {
    margin-left: 70%;
  }

  .main-content.with-config,
  .main-content.with-llama-config {
    margin-right: 70%;
  }

  .main-content.with-terminal.with-config,
  .main-content.with-terminal.with-llama-config {
    margin-left: 70%;
    margin-right: 70%;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary, #666);
    text-align: center;
    min-height: 80vh;
    transition: color 0.3s ease;
  }

  .empty-state p {
    margin: 0.5rem 0;
  }

  .empty-state .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary, #999);
    transition: color 0.3s ease;
  }

  .empty-state .hint-small {
    font-size: 0.8rem;
    color: var(--text-tertiary, #aaa);
    transition: color 0.3s ease;
  }

  @media screen and (max-width: 768px) {
    .ai-chat {
      min-height: 70vh;
      padding: 1rem;
    }

    .main-content {
      max-width: 100%;
      border-radius: 8px;
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
