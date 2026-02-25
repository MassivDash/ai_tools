<script lang="ts">
  import Badge from '../../../ui/Badge.svelte'
  import type { GameStateSnapshot } from '../../../../hooks/useOneOfFifteenState.svelte'

  interface Props {
    contestants: GameStateSnapshot['contestants']
    round: string
  }
  let { contestants, round }: Props = $props()
</script>

<div class="contestants-list">
  <h3>
    Contestants ({contestants.length})
  </h3>
  <ul>
    {#each contestants as contestant}
      <li
        class:online={contestant.online}
        class:offline={!contestant.online}
        class:eliminated={contestant.eliminated}
      >
        <div class="c-info">
          <span class="c-name">
            {contestant.name}
            {#if contestant.age}
              <span class="c-age">({contestant.age})</span>
            {/if}
          </span>
          {#if contestant.eliminated}
            <Badge variant="danger">ELIMINATED</Badge>
          {:else if contestant.ready && round === 'lobby'}
            <Badge variant="success">READY</Badge>
          {/if}
        </div>

        <div class="c-stats">
          {#if round === 'round1'}
            <span class="stat-pill" title="Misses"
              >❌ {contestant.round1_misses}/2</span
            >
          {/if}
          <span class="stat-pill">❤️ {contestant.lives}</span>
          <span class="stat-pill">⭐ {contestant.score}</span>
        </div>
      </li>
    {/each}
  </ul>
</div>

<style>
  .contestants-list {
    text-align: left;
    background: var(--bg-secondary);
    padding: 1rem;
    border-radius: 8px;
    height: 100%;
    border: 1px solid var(--border-color);
    overflow-y: auto;
  }

  ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-primary);
  }
  li.eliminated {
    opacity: 0.5;
    background: var(--bg-secondary);
    /* No text-decoration here because Badge handles the status, but original had it. Keeping it clean or adding it? */
    /* Original had text-decoration line-through. I'll re-add it if needed but maybe better not to strike through name if badge is there. */
    /* Original: text-decoration: line-through; */
  }
  /* Adding line-through back to name if preferred, but Badge is clear enough. */

  li.offline {
    opacity: 0.6;
    background: var(--bg-secondary);
  }

  .c-name {
    font-weight: 600;
  }

  .c-age {
    font-weight: 400;
    color: var(--text-secondary);
    margin-left: 0.5rem;
    font-size: 0.9em;
  }

  .c-stats {
    display: flex;
    gap: 0.5rem;
  }
  .stat-pill {
    background: var(--bg-secondary);
    padding: 0.25rem 0.5rem;
    border-radius: 12px;
    font-size: 0.85rem;
    font-weight: bold;
    border: 1px solid var(--border-color);
  }
</style>
