<script lang="ts">
  import type { Contestant } from '../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    players: Contestant[]
    onSelect: (_id: string) => void
    disabled?: boolean
    excludeId?: string
  }

  let { players, onSelect, disabled = false, excludeId }: Props = $props()

  let eligiblePlayers = $derived(
    players.filter((p) => !p.eliminated && p.online && p.id !== excludeId)
  )
</script>

<div class="player-grid">
  {#if eligiblePlayers.length === 0}
    <p class="no-players">No eligible players to point to.</p>
  {:else}
    {#each eligiblePlayers as player (player.id)}
      <button
        class="player-card"
        onclick={() => {
          onSelect(player.id)
        }}
        {disabled}
      >
        <span class="player-name">{player.name}</span>
        <span class="player-age">Age: {player.age}</span>
        <span class="player-lives">Lives: {player.lives}</span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .player-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 1rem;
    width: 100%;
    margin-top: 1rem;
  }

  .player-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: var(--surface-2);
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    padding: 1rem;
    cursor: pointer;
    transition: all 0.2s ease;
    color: var(--text-1);
  }

  .player-card:hover:not(:disabled) {
    border-color: var(--primary);
    background: var(--surface-3);
    transform: translateY(-2px);
  }

  .player-card:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .player-name {
    font-weight: bold;
    font-size: 1.1rem;
    margin-bottom: 0.25rem;
  }

  .player-age,
  .player-lives {
    font-size: 0.9rem;
    color: var(--text-2);
  }

  .no-players {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--text-2);
  }
</style>
