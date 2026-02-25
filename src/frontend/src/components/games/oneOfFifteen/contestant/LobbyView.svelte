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
    <div class="ready-status-box">
      <div class="icon-pulse">
        <MaterialIcon name="check-circle" width="64" height="64" />
      </div>
      <h3>You are Ready!</h3>
      <p>Waiting for the game to start...</p>
      <button class="btn-not-ready" onclick={onToggleReady}>
        <MaterialIcon name="close" width="20" height="20" />
        Cancel Ready
      </button>
    </div>
  {:else}
    <div class="ready-prompt">
      <h3>Are you ready to play?</h3>
      <p>Click the button below when you are ready.</p>
      <button class="btn-ready" onclick={onToggleReady}>
        <MaterialIcon name="check" width="24" height="24" />
        I'M READY!
      </button>
    </div>
  {/if}
</div>

<style>
  .ready-status-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    color: var(--success);
  }

  .icon-pulse {
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0% {
      transform: scale(1);
      opacity: 1;
    }
    50% {
      transform: scale(1.1);
      opacity: 0.8;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }

  .ready-prompt {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: center;
  }

  .btn-not-ready {
    background: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    cursor: pointer;
    font-size: 1rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition: all 0.2s;
  }
  .btn-not-ready:hover {
    background: var(--bg-secondary);
    border-color: var(--danger);
    color: var(--danger);
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
    display: flex;
    align-items: center;
    gap: 0.5rem;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }
  .btn-ready:active {
    transform: scale(0.95);
  }
</style>
