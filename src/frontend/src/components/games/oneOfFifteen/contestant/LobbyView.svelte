<script lang="ts">
  import MaterialIcon from '../../../ui/MaterialIcon.svelte'

  interface Props {
    isReady: boolean
    onToggleReady: () => void
  }

  let { isReady, onToggleReady }: Props = $props()
</script>

<div class="waiting-screen">
  {#if isReady}
    <div class="spinner-box">
      <MaterialIcon name="clock-outline" width="48" height="48" class="spin" />
    </div>
    <h3>Waiting for Presenter to start...</h3>
    <p class="status-sub">You are ready!</p>
    <button class="btn-link" onclick={onToggleReady}>Not Ready?</button>
  {:else}
    <div class="ready-prompt">
      <h3>Are you ready to play?</h3>
      <p>Click the button below when you are ready.</p>
      <button class="btn-ready" onclick={onToggleReady}> I'M READY! </button>
    </div>
  {/if}
</div>

<style>
  .waiting-screen {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .spinner-box {
    margin-bottom: 1rem;
    color: var(--text-secondary);
  }
  .ready-prompt {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: center;
  }
  .status-sub {
    color: var(--success);
    font-weight: bold;
  }
  .btn-link {
    background: none;
    border: none;
    text-decoration: underline;
    color: var(--text-secondary);
    cursor: pointer;
    margin-top: 1rem;
  }
  .btn-ready {
    background: var(--success);
    color: var(--text-primary-inverse, #fff);
    border: none;
    padding: 1rem 2rem;
    font-size: 1.2rem;
    font-weight: bold;
    border-radius: 8px;
    cursor: pointer;
    transition: transform 0.1s;
  }
  .btn-ready:active {
    transform: scale(0.95);
  }
</style>
