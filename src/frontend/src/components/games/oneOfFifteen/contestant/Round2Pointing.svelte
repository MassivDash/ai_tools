<script lang="ts">
  import PlayerGrid from '../PlayerGrid.svelte'
  import MaterialIcon from '../../../ui/MaterialIcon.svelte'
  import type { Contestant } from '../../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    isMyTurnToPoint: boolean
    players: Contestant[]
    myId: string
    pointerName: string
    onPointToPlayer: (id: string) => void
  }

  let { isMyTurnToPoint, players, myId, pointerName, onPointToPlayer }: Props =
    $props()
</script>

{#if isMyTurnToPoint}
  <!-- Round 2 Pointing UI -->
  <div class="pointing-container">
    <h3>It's your turn to choose the next player!</h3>
    <PlayerGrid
      {players}
      excludeId={myId}
      onSelect={(id) => {
        onPointToPlayer(id)
      }}
    />
  </div>
{:else}
  <!-- Spectating Pointing -->
  <div class="spectator-view">
    <div class="generating-message">
      <MaterialIcon name="account-search-outline" width="48" height="48" />
      <h3>Pointing Phase</h3>
      <p>Waiting for {pointerName} to select a player...</p>
    </div>
  </div>
{/if}

<style>
  .pointing-container {
    background: var(--bg-secondary);
    padding: 2rem;
    border-radius: 12px;
    border: 2px solid var(--accent);
    width: 100%;
    max-width: 800px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
    animation: slideUp 0.3s ease-out;
  }

  .spectator-view {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 100%;
  }

  .generating-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem;
    opacity: 0.7;
  }

  @keyframes slideUp {
    from {
      transform: translateY(20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }
</style>
