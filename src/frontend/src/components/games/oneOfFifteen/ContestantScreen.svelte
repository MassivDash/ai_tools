<script lang="ts">
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    state: GameStateSnapshot
    contestantName: string
    sessionId: string
    onToggleReady: () => void
  }

  let { state, contestantName, sessionId, onToggleReady }: Props = $props()

  let myContestant = $derived(
    state.contestants.find(
      (c) => c.id === sessionId || c.session_id === sessionId
    )
  )
  let isReady = $derived(myContestant?.ready || false)
</script>

<div class="contestant-dashboard">
  <h2>Welcome, {contestantName || myContestant?.name || 'Player'}!</h2>

  {#if state.status === 'lobby'}
    <div class="waiting-screen">
      {#if isReady}
        <div class="spinner-box">
          <MaterialIcon
            name="clock-outline"
            width="48"
            height="48"
            class="spin"
          />
        </div>
        <h3>Waiting for Presenter to start...</h3>
        <p class="status-sub">You are ready!</p>
        <button class="btn-link" onclick={onToggleReady}>Not Ready?</button>
      {:else}
        <div class="ready-prompt">
          <h3>Are you ready to play?</h3>
          <p>Click the button below when you are ready.</p>
          <button class="btn-ready" onclick={onToggleReady}>
            I'M READY!
          </button>
        </div>
      {/if}
    </div>
  {:else if state.status === 'playing'}
    <div class="game-active-screen">
      <h3>Game Started!</h3>
      <p>Get ready for the first question...</p>
      <!-- Question UI will go here -->

      <div class="score-display">
        <span class="score-label">Your Score</span>
        <span class="score-value">0</span>
      </div>
    </div>
  {:else}
    <!-- Finished or other state -->
    <div class="waiting-screen">
      <h3>Game Ended.</h3>
    </div>
  {/if}

  {#if state.has_presenter}
    <div
      class="presenter-status {state.presenter_online ? 'online' : 'offline'}"
    >
      {#if state.presenter_online}
        <MaterialIcon name="check-circle" width="16" height="16" /> Presenter Online
      {:else}
        <MaterialIcon name="alert-circle" width="16" height="16" /> Presenter Offline
      {/if}
    </div>
  {:else}
    <div class="presenter-status offline">
      <MaterialIcon name="alert-circle" width="16" height="16" /> Presenter Offline
    </div>
  {/if}
</div>

<style>
  .contestant-dashboard {
    text-align: center;
    padding: 2rem;
  }

  .score-display {
    margin-top: 2rem;
    padding: 2rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .score-label {
    font-size: 1.2rem;
    color: var(--text-secondary, #666);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .score-value {
    font-size: 4rem;
    font-weight: 700;
    color: var(--primary-color, #2196f3);
    line-height: 1;
  }

  /* Contestant specific */
  .waiting-screen,
  .game-active-screen {
    margin: 2rem 0;
    padding: 2rem;
    background: var(--bg-secondary, #f9f9f9);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .spin {
    animation: spin 2s linear infinite;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .presenter-status {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 20px;
    margin: 1rem 0;
    font-weight: 500;
    font-size: 0.9rem;
  }
  .presenter-status.online {
    background-color: #e8f5e9;
    color: #2e7d32;
  }
  .presenter-status.offline {
    background-color: #ffebee;
    color: #c62828;
  }

  .btn-ready {
    background: #4caf50;
    color: white;
    border: none;
    padding: 1rem 2rem;
    font-size: 1.5rem;
    font-weight: bold;
    border-radius: 50px;
    cursor: pointer;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    transition:
      transform 0.1s,
      box-shadow 0.1s;
    margin-top: 1rem;
  }
  .btn-ready:active {
    transform: scale(0.98);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }
  .btn-link {
    background: none;
    border: none;
    color: var(--text-secondary, #666);
    text-decoration: underline;
    cursor: pointer;
    margin-top: 1rem;
    font-size: 0.9rem;
  }
  .status-sub {
    color: #4caf50;
    font-weight: bold;
  }
</style>
