<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import ServerControls from '../agent/ServerControls.svelte'
  import Terminal from '../llamaServer/terminal.svelte'
  import LlamaConfig from '../llamaServer/config/LlamaConfig.svelte'
  import { useStatusWebSocket } from '../../hooks/useStatusWebSocket'
  import type { LlamaServerStatus, LlamaServerResponse } from '../agent/types'

  // --- Server State ---
  let serverStatus: LlamaServerStatus = $state({ active: false, port: 8080 })
  let serverLoading = $state(false)
  let showConfig = $state(false)
  let showTerminal = $state(false)

  let error = $state('')

  // --- WebSocket for Server Status ---
  const statusWs = useStatusWebSocket(
    (status) => {
      serverStatus = status
    },
    () => {
      // Auto-close terminal when server is ready, similar to Agent
      showTerminal = false
    }
  )

  onMount(() => {
    statusWs.connect()
  })

  onDestroy(() => {
    statusWs.disconnect()
  })

  // --- Server Controls ---
  const startServer = async () => {
    if (serverStatus.active) return
    serverLoading = true
    error = ''
    showTerminal = true
    try {
      const response =
        await axiosBackendInstance.post<LlamaServerResponse>(
          'llama-server/start'
        )
      if (!response.data.success) {
        error = response.data.message
      }
    } catch (err: any) {
      error = err.message || 'Failed to start server'
    } finally {
      serverLoading = false
    }
  }

  const stopServer = async () => {
    serverLoading = true
    try {
      const response =
        await axiosBackendInstance.post<LlamaServerResponse>(
          'llama-server/stop'
        )
      if (response.data.success) {
        serverStatus.active = false
      }
    } catch (err: any) {
      error = err.message
    } finally {
      serverLoading = false
    }
  }
</script>

<PageSubHeader title="1 of 15" icon="gamepad-variant">
  {#snippet leftContent()}
    <Button variant="ghost" href="/games" size="small">
      <MaterialIcon name="arrow-left" width="20" height="20" />
    </Button>
  {/snippet}
  {#snippet actions()}
    <div class="server-controls-wrapper">
      <ServerControls
        serverActive={serverStatus.active}
        loading={serverLoading}
        onStart={startServer}
        onStop={stopServer}
      />
    </div>

    <Button
      variant="info"
      class="button-icon-only"
      onclick={() => (showConfig = !showConfig)}
      title="Game Config"
    >
      <MaterialIcon name="cog" width="32" height="32" />
    </Button>

    <Button
      variant="info"
      class="button-icon-only"
      onclick={() => (showTerminal = !showTerminal)}
      title={showTerminal ? 'Hide Terminal' : 'Show Terminal'}
    >
      <MaterialIcon name="console" width="32" height="32" />
    </Button>
  {/snippet}
</PageSubHeader>

<div class="game-container">
  <!-- Sliding Terminal Sidebar (Left) -->
  <div class="terminal-sidebar" class:visible={showTerminal}>
    <Terminal />
  </div>

  <!-- Main Content Area -->
  <div
    class="content-area"
    class:with-terminal={showTerminal}
    class:with-config={showConfig}
  >
    {#if error}
      <div class="error-banner">
        <MaterialIcon name="alert-circle" width="24" height="24" />
        <span>{error}</span>
      </div>
    {/if}

    <div class="game-lobby">
      <div class="lobby-card">
        <MaterialIcon
          name="gamepad-variant"
          width="80"
          height="80"
          class="game-logo"
        />
        <h1>1 of 15</h1>
        <p class="description">
          A general knowledge quiz game hosted by an AI personality. Prepare to
          answer 15 questions correctly to win!
        </p>

        <div class="status-box">
          {#if serverStatus.active}
            <div class="status-ready">
              <MaterialIcon name="check-circle" width="24" height="24" />
              <span>Game Server Ready</span>
            </div>
          {:else}
            <div class="status-waiting">
              <MaterialIcon name="server-network-off" width="24" height="24" />
              <span>Server Offline</span>
            </div>
            <p class="hint">
              Start the server using the controls above to play.
            </p>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- Sliding Config Panel (Right) -->
  <LlamaConfig isOpen={showConfig} onClose={() => (showConfig = false)} />
</div>

<style>
  .game-container {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 140px); /* Adjust for headers */
    overflow: hidden;
    position: relative;
    background-color: var(--bg-primary, #fff);
  }

  .server-controls-wrapper {
    margin-right: 1rem;
    display: flex;
    align-items: center;
  }

  /* Sliding Terminal Styles */
  .terminal-sidebar {
    width: 60%;
    height: 100%;
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
    box-shadow: 2px 0 10px rgba(0, 0, 0, 0.2);
  }

  .terminal-sidebar.visible {
    transform: translateX(0);
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    transition:
      margin 0.3s ease-in-out,
      width 0.3s ease-in-out;
    margin-left: 0;
    margin-right: 0;
    width: 100%;
    background-color: var(--bg-primary, #fff);
    position: relative;
    overflow-y: auto;
  }

  .content-area.with-terminal {
    margin-left: 60%;
    width: 40%;
  }

  .content-area.with-config {
    margin-right: 70%; /* Match LlamaConfig width */
    width: 30%; /* Approximate remaining space */
  }

  .content-area.with-terminal.with-config {
    margin-left: 60%;
    margin-right: 70%;
    width: 0; /* Squeezed out */
  }

  /* On mobile/tablet, behavior might need adjustment (overlay) */

  .error-banner {
    background-color: #ffebee;
    color: #c62828;
    padding: 1rem;
    margin: 1rem;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .game-lobby {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }

  .lobby-card {
    background-color: var(--bg-secondary, #fafafa);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 16px;
    padding: 3rem;
    max-width: 600px;
    width: 100%;
    text-align: center;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.05);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
  }

  h1 {
    margin: 0;
    font-size: 2.5rem;
    color: var(--text-primary, #333);
  }

  .description {
    color: var(--text-secondary, #666);
    font-size: 1.1rem;
    line-height: 1.6;
    margin: 0;
  }

  .status-box {
    margin-top: 1rem;
    padding: 1.5rem;
    border-radius: 12px;
    background-color: var(--bg-primary, #fff);
    width: 100%;
    border: 1px solid var(--border-color, #e0e0e0);
  }

  .status-ready {
    color: var(--success-color, #4caf50);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 600;
    font-size: 1.2rem;
  }

  .status-waiting {
    color: var(--text-secondary, #bbb);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 500;
    font-size: 1.2rem;
  }

  .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary, #999);
    margin-top: 0.5rem;
  }
</style>
