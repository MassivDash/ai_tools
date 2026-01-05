<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import Input from '../ui/Input.svelte'
  import ServerControls from '../agent/config/ServerControls.svelte'
  import Terminal from '../llamaServer/terminal.svelte'
  import LlamaConfig from '../llamaServer/config/LlamaConfig.svelte'
  import { useStatusWebSocket } from '../../hooks/useStatusWebSocket'
  import { useOneOfFifteenState } from '../../hooks/useOneOfFifteenState.svelte'
  import type { LlamaServerStatus, LlamaServerResponse } from '@types'
  import PresenterScreen from './oneOfFifteen/PresenterScreen.svelte'
  import ContestantScreen from './oneOfFifteen/ContestantScreen.svelte'
  import { ContestantJoinSchema } from '../../validation/oneOfFifteen'

  // --- Server State ---
  let serverStatus: LlamaServerStatus = $state({ active: false, port: 8080 })
  let serverLoading = $state(false)
  let showConfig = $state(false)
  let showTerminal = $state(false)

  // --- Game State (via Hook) ---
  const game = useOneOfFifteenState()
  let contestantName = $state('')
  let contestantAge = $state('')
  // We use game.state.role, game.state.gameState, game.isConnected

  // Computed helpers
  let joined = $derived(!!game.state.role)
  let role = $derived(game.state.role)
  let isFormValid = $derived(
    ContestantJoinSchema.safeParse({
      name: contestantName,
      age: contestantAge
    }).success
  )
  // Local UI state
  let selectingContestant = $state(false)

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

  $effect(() => {
    if (serverStatus.active && !game.isConnected) {
      game.connect()
    } else if (!serverStatus.active && game.isConnected) {
      game.disconnect()
    }
  })

  onMount(() => {
    statusWs.connect()
  })

  onDestroy(() => {
    statusWs.disconnect()
    game.disconnect()
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

    {#if joined}
      <Button
        variant="danger"
        class="button-icon-only"
        onclick={game.logout}
        title="Leave Game (Clear Session)"
      >
        <MaterialIcon name="logout" width="24" height="24" />
      </Button>
    {/if}

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
      {#if !serverStatus.active}
        <div class="lobby-card">
          <MaterialIcon
            name="gamepad-variant"
            width="80"
            height="80"
            class="game-logo"
          />
          <h1>1 of 15</h1>
          <p class="description">
            A general knowledge quiz game hosted by an AI personality. Start the
            server to begin.
          </p>

          <div class="status-box">
            <div class="status-waiting">
              <MaterialIcon name="server-network-off" width="24" height="24" />
              <span>Server Offline</span>
            </div>
            <p class="hint">
              Start the server using the controls above to play.
            </p>
          </div>
        </div>
      {:else if !joined}
        <!-- Setup / Role Selection -->
        <div class="setup-container">
          {#if role === null && !selectingContestant}
            <h2>Choose Your Role</h2>
            <div class="role-selection">
              <Button
                variant="primary"
                class="role-card"
                disabled={game.state.gameState.has_presenter}
                title={game.state.gameState.has_presenter
                  ? 'Presenter already needed'
                  : 'Become the Presenter'}
                onclick={() => {
                  game.joinPresenter()
                  // Allow UI to update from hook state
                }}
              >
                <div class="role-content">
                  <MaterialIcon
                    name="monitor-dashboard"
                    width="48"
                    height="48"
                  />
                  <span class="role-title">Presenter</span>
                  <span class="role-desc">Main screen, questions, controls</span
                  >
                </div>
              </Button>

              <Button
                variant="secondary"
                class="role-card"
                onclick={() => {
                  selectingContestant = true
                }}
              >
                <div class="role-content">
                  <MaterialIcon name="account" width="48" height="48" />
                  <span class="role-title">Contestant</span>
                  <span class="role-desc">Join the game, answer questions</span>
                </div>
              </Button>
            </div>
          {:else if selectingContestant}
            <div class="name-entry-card">
              <h2>Enter Details</h2>
              <div class="input-group">
                <Input
                  placeholder="Your Name"
                  bind:value={contestantName}
                  autofocus
                />
                <Input
                  placeholder="Age"
                  bind:value={contestantAge}
                  type="number"
                />
              </div>
              <div class="actions">
                <Button
                  variant="ghost"
                  onclick={() => (selectingContestant = false)}
                >
                  Back
                </Button>
                <Button
                  variant="primary"
                  disabled={!isFormValid}
                  onclick={() => {
                    const result = ContestantJoinSchema.safeParse({
                      name: contestantName,
                      age: contestantAge
                    })
                    if (result.success) {
                      game.joinContestant(
                        result.data.name,
                        result.data.age.toString()
                      )
                    }
                    // Hook updates state.role -> 'contestant', which hides this view
                  }}
                >
                  Join Game
                </Button>
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <!-- Game View -->
        <div class="game-view">
          {#if role === 'presenter'}
            <PresenterScreen
              gameState={game.state.gameState}
              onStartGame={game.startGame}
              onResetGame={game.resetGame}
            />
          {:else}
            <ContestantScreen
              gameState={game.state.gameState}
              {contestantName}
              sessionId={game.sessionId}
              onToggleReady={game.toggleReady}
              onSubmitAnswer={game.submitAnswer}
            />
          {/if}
        </div>
      {/if}
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
    background-color: var(--bg-terminal, #1e1e1e);
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

  /* --- Game Setup Styles --- */
  .setup-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2rem;
    width: 100%;
    max-width: 800px;
  }

  .setup-container h2 {
    font-size: 2rem;
    margin: 0;
    color: var(--text-primary, #333);
  }

  .role-selection {
    display: flex;
    gap: 2rem;
    flex-wrap: wrap;
    justify-content: center;
    width: 100%;
  }

  /* Custom styling for Buttons to make them look like cards */
  :global(.role-card) {
    height: auto !important;
    padding: 2rem !important;
    border-radius: 16px !important;
    flex: 1;
    min-width: 250px;
    max-width: 350px;
    transition: transform 0.2s;
  }

  :global(.role-card:hover) {
    transform: translateY(-4px);
  }

  .role-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    text-align: center;
  }

  .role-title {
    font-size: 1.5rem;
    font-weight: 600;
  }

  .role-desc {
    font-size: 1rem;
    opacity: 0.9;
    font-weight: 400;
  }

  .name-entry-card {
    background-color: var(--bg-secondary, #fafafa);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 16px;
    padding: 3rem;
    width: 100%;
    max-width: 500px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2rem;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.05);
  }

  .input-group {
    width: 100%;
  }

  .actions {
    display: flex;
    gap: 1rem;
    width: 100%;
    justify-content: flex-end;
  }

  /* --- In-Game Views --- */
  .game-view {
    width: 100%;
    max-width: 1000px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  /* Presenter specific styles - MOVED TO SUBCOMPONENTS */
</style>
