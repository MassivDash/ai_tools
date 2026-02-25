<script lang="ts">
  interface Props {
    score: number
    lives: number
    isRound1: boolean
    round1Misses?: number
  }

  let { score, lives, isRound1, round1Misses = 0 }: Props = $props()
</script>

<div class="stats-footer">
  <div class="stat-box">
    <span class="label">Score</span>
    <span class="value">{score}</span>
  </div>
  <div class="stat-box">
    <span class="label">Lives</span>
    <span class="value">{'❤️'.repeat(Math.max(0, lives))}</span>
  </div>
  {#if isRound1}
    <div class="stat-box">
      <span class="label">Strikes</span>
      <span class="value">{round1Misses}/2</span>
    </div>
  {/if}
</div>

<style>
  .stats-footer {
    position: fixed;
    bottom: 0;
    left: 0;
    width: 100%;
    background: var(--bg-primary);
    border-top: 1px solid var(--border-color);
    display: flex;
    justify-content: space-around;
    padding: 1rem;
    box-shadow: 0 -2px 10px rgba(0, 0, 0, 0.1);
    z-index: 10;
  }
  .stat-box {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .stat-box .label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
  .stat-box .value {
    font-weight: bold;
    font-size: 1.25rem;
    margin-top: 0.25rem;
    color: var(--text-primary);
  }
</style>
