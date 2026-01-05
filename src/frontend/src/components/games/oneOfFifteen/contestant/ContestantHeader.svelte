<script lang="ts">
  import MaterialIcon from '../../../ui/MaterialIcon.svelte'

  interface Props {
    contestantName: string
    statusMessage: string
    isActivePlayer: boolean
    isEliminated: boolean
    hasPresenter: boolean
    presenterOnline: boolean
  }

  let {
    contestantName,
    statusMessage,
    isActivePlayer,
    isEliminated,
    hasPresenter,
    presenterOnline
  }: Props = $props()
</script>

<div class="contestant-head-wrapper">
  <h2>Welcome, {contestantName || 'Player'}!</h2>

  <div class="header-status">
    <div
      class="status-badge {isActivePlayer ? 'active' : ''} {isEliminated
        ? 'eliminated'
        : ''}"
    >
      {statusMessage}
    </div>
  </div>

  {#if hasPresenter}
    <div class="presenter-status {presenterOnline ? 'online' : 'offline'}">
      {#if presenterOnline}
        <MaterialIcon name="check-circle" width="16" height="16" /> Presenter Online
      {:else}
        <MaterialIcon name="alert-circle" width="16" height="16" /> Presenter Offline
      {/if}
    </div>
  {:else}
    <div class="presenter-status offline">
      <MaterialIcon name="alert-circle" width="16" height="16" /> Presenter Offline
    </div>
  {/if}
</div>

<style>
  .contestant-head-wrapper {
    width: 100%;
    margin-bottom: 1rem;
    position: relative;
  }

  .header-status {
    margin-bottom: 1rem;
  }
  .status-badge {
    display: inline-block;
    padding: 0.5rem 1rem;
    border-radius: 999px;
    background: var(--surface-2);
    font-weight: bold;
    font-size: 0.9rem;
  }
  .status-badge.active {
    background: var(--primary);
    color: var(--text-primary-inverse);
  }
  .status-badge.eliminated {
    background: var(--error);
    color: var(--text-primary-inverse);
  }

  .presenter-status {
    position: absolute;
    top: -1rem;
    right: -1rem;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 16px;
  }
  .presenter-status.online {
    color: var(--success);
  }
  .presenter-status.offline {
    color: var(--error);
  }
</style>
