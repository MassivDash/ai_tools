<script lang="ts">
  import PlayerGrid from '../PlayerGrid.svelte'
  import type { Contestant } from '../../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    players: Contestant[]
    myId: string
    onMakeDecision: (_choice: 'self' | 'point', _targetId?: string) => void
  }

  let { players, myId, onMakeDecision }: Props = $props()

  let showGrid = $state(false)
</script>

<div class="decision-container">
  <h3>Correct! What do you want to do?</h3>
  {#if !showGrid}
    <div class="decision-buttons">
      <button class="btn-decision self" onclick={() => onMakeDecision('self')}>
        Double Down (Self)
      </button>
      <button class="btn-decision point" onclick={() => (showGrid = true)}>
        Point to Player
      </button>
    </div>
  {:else}
    <!-- Choosing a player to point to -->
    <div class="pointing-container">
      <h4>Select a player to answer:</h4>
      <PlayerGrid
        {players}
        excludeId={myId}
        onSelect={(id) => onMakeDecision('point', id)}
      />
      <button class="btn-link" onclick={() => (showGrid = false)}>Back</button>
    </div>
  {/if}
</div>

<style>
  .decision-container {
    background: var(--bg-secondary);
    padding: 2rem;
    border-radius: 12px;
    border: 2px solid var(--warning);
    width: 100%;
    max-width: 800px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
    animation: slideUp 0.3s ease-out;
  }

  .decision-buttons {
    display: flex;
    gap: 2rem;
  }
  .btn-decision {
    padding: 1.5rem 2rem;
    font-size: 1.2rem;
    border-radius: 8px;
    border: none;
    cursor: pointer;
    font-weight: bold;
    color: white;
  }
  .btn-decision.self {
    background: var(--primary);
  }
  .btn-decision.point {
    background: var(--accent);
  }

  .pointing-container {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .btn-link {
    background: none;
    border: none;
    text-decoration: underline;
    color: var(--text-secondary);
    cursor: pointer;
    margin-top: 1rem;
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
