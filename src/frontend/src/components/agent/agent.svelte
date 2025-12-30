<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Terminal from '../llamaServer/terminal.svelte'
  import AgentConfig from './agentConfig.svelte'
  import LlamaConfig from '../llamaServer/llamaConfig.svelte'
  import ChatInterface from './chatInterface.svelte'
  import { useStatusWebSocket } from '../../hooks/useStatusWebSocket'
  import { enabledTools as enabledToolsStore } from '../../stores/activeTools'
  import type {
    LlamaServerStatus,
    LlamaServerResponse,
    AgentConfig as AgentConfigType
  } from './types'
  import AgentHeader from './AgentHeader.svelte'
  import ServerControls from './ServerControls.svelte'
  import EmptyState from './EmptyState.svelte'
  import HistorySidebar from './HistorySidebar.svelte'

  import TestingSidebar from './TestingSidebar.svelte'

  let serverStatus: LlamaServerStatus = { active: false, port: 8080 }
  let loading = false
  let error = ''
  let showConfig = false
  let showLlamaConfig = false
  let showTerminal = false
  let showHistory = false
  let showTesting = false
  let _isStarting = false

  // Refs
  let chatInterface: ChatInterface
  let testingSidebar: TestingSidebar
  let chatLoading = false

  // Current conversation state
  let currentConversationId: string | undefined = undefined

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

  $: if (!chatLoading && showTesting && testingSidebar) {
    // Potentially trigger next question if runner is active
    // We need a clearer signal than just loading false, but for now this is the hook.
    // Better: pass chatLoading to sidebar or call method?
    // TestingSidebar exposes handleRunnerNext() which checks internal running state.
    // So it is safe to call repeatedly?
    // handleRunnerNext checks: if (running && index < length - 1)
    // We should only call it once when loading transitions from true to false.
    // logic below.
  }

  let prevChatLoading = false
  let runnerDebounceTimer: any

  $: {
    if (prevChatLoading && !chatLoading && showTesting && testingSidebar) {
      // Clear any existing timer to avoid stacking
      if (runnerDebounceTimer) clearTimeout(runnerDebounceTimer)

      // Debounce the next question trigger slightly more and ensure single execution
      runnerDebounceTimer = setTimeout(() => {
        testingSidebar.handleRunnerNext()
        runnerDebounceTimer = null
      }, 500)
    }
    prevChatLoading = chatLoading
  }

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
        await axiosBackendInstance.get<AgentConfigType>('agent/config')
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

  const handleToggleConfig = () => {
    showConfig = !showConfig
    if (showConfig) {
      showLlamaConfig = false
      showTesting = false
    }
  }

  const handleToggleLlamaConfig = () => {
    showLlamaConfig = !showLlamaConfig
    if (showLlamaConfig) {
      showConfig = false
      showTesting = false
    }
  }

  const handleToggleTesting = () => {
    showTesting = !showTesting
    if (showTesting) {
      showConfig = false
      showLlamaConfig = false
      showHistory = false // Maybe overlay or sidebar conflict?
      // Keep terminal if desired, but sidebar space is limited.
    }
  }

  const handleToggleTerminal = () => {
    showTerminal = !showTerminal
    if (showTerminal) showHistory = false
  }

  const handleToggleHistory = () => {
    showHistory = !showHistory
    if (showHistory) {
      showTerminal = false
      showTesting = false
    }
  }

  const handleSelectConversation = (event: CustomEvent<string>) => {
    currentConversationId = event.detail
    // Logic to reload chat for this ID will be in ChatInterface (via prop)
  }

  const handleNewConversation = () => {
    currentConversationId = undefined
  }

  const handleConversationCreated = (event: CustomEvent<string>) => {
    const newId = event.detail
    currentConversationId = newId
    // Refresh history sidebar logic?
    // We can force sidebar refresh by keying it or calling a method.
    // Or just let it refresh on mount/open. But better to refresh now.
    // Since sidebar component loads on mount/open, we might need a way to tell it to reload.
    // We can add a refresh trigger prop or call a method if we bind to it.
    // For now let's use a simple reactive statement in sidebar or just ignore?
    // User complaint: "history screen is not updating with new conversation"
    // So we MUST refresh sidebar.
    shouldRefreshHistory = true
    setTimeout(() => (shouldRefreshHistory = false), 100)
  }

  const handleRunQuestion = (event: CustomEvent<{ content: string }>) => {
    const content = event.detail.content
    if (chatInterface) {
      chatInterface.sendMessage(content)
    }
  }

  let shouldRefreshHistory = false

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
  <AgentHeader
    {showConfig}
    {showLlamaConfig}
    {showTerminal}
    {showHistory}
    {showTesting}
    onToggleConfig={handleToggleConfig}
    onToggleLlamaConfig={handleToggleLlamaConfig}
    onToggleTerminal={handleToggleTerminal}
    onToggleHistory={handleToggleHistory}
    onToggleTesting={handleToggleTesting}
  >
    <ServerControls
      serverActive={serverStatus.active}
      {loading}
      onStart={startServer}
      onStop={stopServer}
    />
  </AgentHeader>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div
    class="content-area"
    class:has-terminal={showTerminal}
    class:has-history={showHistory}
    class:has-config={showConfig}
    class:has-llama-config={showLlamaConfig}
    class:has-testing={showTesting}
  >
    <HistorySidebar
      isOpen={showHistory}
      {currentConversationId}
      shouldRefresh={shouldRefreshHistory}
      on:select={handleSelectConversation}
      on:new={handleNewConversation}
      on:close={() => (showHistory = false)}
    />

    <TestingSidebar
      bind:this={testingSidebar}
      isOpen={showTesting}
      on:close={() => (showTesting = false)}
      on:runQuestion={handleRunQuestion}
    />

    <div class="terminal-sidebar" class:visible={showTerminal}>
      <Terminal />
    </div>

    <div
      class="main-content"
      class:with-terminal={showTerminal}
      class:with-history={showHistory}
      class:with-config={showConfig}
      class:with-llama-config={showLlamaConfig}
      class:with-testing={showTesting}
    >
      {#if serverStatus.active}
        <ChatInterface
          bind:this={chatInterface}
          bind:loading={chatLoading}
          {currentConversationId}
          on:newChat={handleNewConversation}
          on:conversationCreated={handleConversationCreated}
          on:responseComplete={(e) => {
            if (showTesting && testingSidebar) {
              testingSidebar.handleResponseMetrics(e.detail)
            }
          }}
        />
      {:else}
        <EmptyState />
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
    display: flex;
    flex-direction: column;
    background-color: var(--bg-primary, #fff);
    transition: background-color 0.3s ease;
    box-sizing: border-box; /* Ensure no scroll on this container */
  }

  .error {
    padding: 0.75rem;
    margin: 0 1rem;
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

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: row;
    height: 100%;
    position: relative;
    overflow: hidden;
    width: 100%;
  }

  .terminal-sidebar {
    width: 60%;
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
    border-radius: 8px;
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
    overflow-y: hidden;
    padding: 0;
    height: 100%; /* Ensure it fills parent */
  }

  .main-content.with-terminal {
    margin-left: 70%;
  }

  .main-content.with-history {
    margin-left: 260px;
  }

  .main-content.with-testing {
    margin-left: 320px;
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

  @media screen and (max-width: 768px) {
    .ai-chat {
      height: 80vh;
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

    .main-content.with-history {
      margin-left: 0; /* Overlay on mobile */
    }

    .main-content.with-testing {
      margin-left: 0;
    }
  }
</style>
