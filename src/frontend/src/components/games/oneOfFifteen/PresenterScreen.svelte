<script lang="ts">
  import Button from '../../ui/Button.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    state: GameStateSnapshot
    onStartGame: () => void
    onResetGame: () => void
  }

  let { state, onStartGame, onResetGame }: Props = $props()
</script>

<div class="presenter-dashboard">
  <div class="presenter-header-banner">
    <MaterialIcon name="monitor-dashboard" width="32" height="32" />
    <h2>Presenter Dashboard</h2>
  </div>

  <div class="dashboard-grid">
    <div class="controls-panel">
      <h3>Game Controls</h3>
      <div class="control-buttons">
        {#if state.status === 'lobby'}
          <Button
            variant="success"
            onclick={onStartGame}
            disabled={state.contestants.length === 0}
          >
            <MaterialIcon name="play" width="24" height="24" />
            Start Game
          </Button>
          <p class="status-text">Status: Lobby (Waiting for players)</p>
        {:else if state.status === 'playing'}
          <Button variant="danger" onclick={onResetGame}>
            <MaterialIcon name="refresh" width="24" height="24" />
            Reset Game
          </Button>
          <p class="status-text active">Status: Game in Progress</p>
        {/if}
      </div>
    </div>

    <div class="contestants-list">
      <h3>
        Contestants ({state.contestants.length})
      </h3>
      <ul>
        {#each state.contestants as contestant}
          <li
            class:online={contestant.online}
            class:offline={!contestant.online}
          >
            <span class="c-name">{contestant.name}</span>
            <div class="c-status">
              {#if contestant.ready}
                <span class="badge u-ready">READY</span>
              {:else}
                <span class="badge u-waiting">WAITING</span>
              {/if}
              <span
                class="badge {contestant.online ? 'u-online' : 'u-offline'}"
              >
                {contestant.online ? 'Online' : 'Offline'}
              </span>
              <span class="score">Score: {contestant.score}</span>
            </div>
          </li>
        {/each}
      </ul>
    </div>
  </div>
</div>

<style>
  .presenter-dashboard {
    text-align: left;
    width: 100%;
  }

  .presenter-header-banner {
    background: var(--bg-secondary, #333);
    color: var(--text-primary-inverse, #fff);
    padding: 1rem 2rem;
    border-radius: 12px;
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 2rem;
  }
  .presenter-header-banner h2 {
    margin: 0;
    font-size: 1.5rem;
  }

  .dashboard-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
  }

  .controls-panel {
    background: var(--bg-secondary, #f5f5f5);
    padding: 1.5rem;
    border-radius: 12px;
  }

  .control-buttons {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: flex-start;
    margin-top: 1rem;
  }

  .status-text {
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
    margin: 0;
  }
  .status-text.active {
    color: #2e7d32;
    font-weight: bold;
  }

  .contestants-list {
    text-align: left;
    background: var(--bg-secondary, #f9f9f9);
    padding: 1rem;
    border-radius: 8px;
    height: 100%;
  }

  .contestants-list ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .contestants-list li {
    padding: 0.75rem;
    background: var(--bg-primary, #fff);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 6px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .contestants-list li.offline {
    opacity: 0.6;
    background: var(--bg-secondary, #f5f5f5);
  }

  .c-name {
    font-weight: 600;
  }

  .c-status {
    display: flex;
    align-items: center;
    gap: 1rem;
    font-size: 0.85rem;
  }

  .badge {
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    text-transform: uppercase;
    font-weight: bold;
  }
  .u-online {
    background: #e8f5e9;
    color: #2e7d32;
  }
  .u-offline {
    background: #ffebee;
    color: #c62828;
  }
  .u-ready {
    background: #e8f5e9;
    color: #2e7d32;
    border: 1px solid #2e7d32;
  }
  .u-waiting {
    background: #fff3e0;
    color: #ef6c00;
    border: 1px solid #ef6c00;
  }
</style>
