<script lang="ts">
  import MaterialIcon from '../../../ui/MaterialIcon.svelte'

  interface Props {
    score: number
    isRound3: boolean
  }

  let { score, isRound3 }: Props = $props()
</script>

<div class="eliminated-view">
  <MaterialIcon name="close-circle" width="64" height="64" />
  <h2>ELIMINATED</h2>
  <p>You have been eliminated from the game.</p>
  {#if isRound3}
    <p>Current Round: The Buzzer!</p>
  {/if}
  <div class="final-score">Final Score: {score}</div>
</div>

<style>
  .eliminated-view {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--error);
  }
  .eliminated-view h2 {
    margin: 1rem 0 0.5rem;
    font-size: 2rem;
  }
  .final-score {
    font-size: 1.5rem;
    font-weight: bold;
    color: var(--text-primary);
  }
</style>
