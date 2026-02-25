<script lang="ts">
  import ContestantHeader from './ContestantHeader.svelte'
  import StatsFooter from './StatsFooter.svelte'
  import type { Snippet } from 'svelte'

  interface Props {
    // Header Props
    contestantName: string
    statusMessage: string
    isActivePlayer: boolean
    isEliminated: boolean
    hasPresenter: boolean
    presenterOnline: boolean

    // Footer Props
    score: number
    lives: number
    isRound1: boolean
    round1Misses?: number
    hideFooter?: boolean

    // Slot
    children: Snippet
  }

  let {
    children,
    contestantName,
    statusMessage,
    isActivePlayer,
    isEliminated,
    hasPresenter,
    presenterOnline,
    score,
    lives,
    isRound1,
    round1Misses,
    hideFooter
  }: Props = $props()
</script>

<div class="contestant-dashboard">
  <ContestantHeader
    {contestantName}
    {statusMessage}
    {isActivePlayer}
    {isEliminated}
    {hasPresenter}
    {presenterOnline}
  />

  <div class="content-area">
    {@render children()}
  </div>

  {#if !hideFooter}
    <StatsFooter {score} {lives} {isRound1} {round1Misses} />
  {/if}
</div>

<style>
  .contestant-dashboard {
    text-align: center;
    padding: 2rem;
    height: 100vh;
    display: flex;
    flex-direction: column;
    color: var(--text-primary);
  }

  .content-area {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center; /* Center everything */
    justify-content: center;
    padding-bottom: 80px; /* Space for footer */
    width: 100%;
  }
</style>
