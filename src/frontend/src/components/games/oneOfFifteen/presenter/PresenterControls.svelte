<script lang="ts">
  import Button from '../../../ui/Button.svelte'
  import MaterialIcon from '../../../ui/MaterialIcon.svelte'

  interface Props {
    round: string
    playerCount: number
    isIntroPlaying: boolean
    onStartGame: () => void
    onResetGame: () => void
  }
  let { round, playerCount, isIntroPlaying, onStartGame, onResetGame }: Props =
    $props()
</script>

<div class="controls-panel">
  <h3>Game Controls</h3>
  <div class="control-buttons">
    {#if round === 'lobby'}
      <Button
        variant="success"
        onclick={onStartGame}
        disabled={playerCount === 0 || isIntroPlaying}
      >
        <MaterialIcon name="play" width="24" height="24" />
        Start Game
      </Button>
      <p class="status-text">
        Status: Lobby ({playerCount} players)
      </p>
    {:else if round !== 'finished'}
      <Button variant="danger" onclick={onResetGame}>
        <MaterialIcon name="refresh" width="24" height="24" />
        Reset Game
      </Button>
      <p class="status-text active">
        Status: {round} Active
      </p>
    {/if}
  </div>
</div>

<style>
  .controls-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    padding: 1rem;
    border-radius: 8px;
    height: 100%;
  }
  .status-text {
    font-size: 0.9rem;
    color: var(--text-secondary);
    margin: 0;
    margin-top: 0.5rem;
  }
  .status-text.active {
    color: var(--success);
    font-weight: bold;
  }
</style>
