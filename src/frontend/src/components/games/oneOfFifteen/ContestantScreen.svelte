<script lang="ts">
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    state: GameStateSnapshot
    contestantName: string
  }

  let { state, contestantName }: Props = $props()
</script>

<div class="contestant-dashboard">
  <h2>Welcome, {contestantName || 'Player'}!</h2>

  {#if state.status === 'lobby'}
    <div class="waiting-screen">
      <div class="spinner-box">
        <MaterialIcon
          name="clock-outline"
          width="48"
          height="48"
          class="spin"
        />
      </div>
      <h3>Waiting for Presenter to start...</h3>
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
    background: #f9f9f9;
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
</style>
