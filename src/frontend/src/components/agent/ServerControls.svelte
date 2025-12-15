<script lang="ts">
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'

  export let serverActive: boolean = false
  export let loading: boolean = false
  export let onStart: () => void
  export let onStop: () => void
</script>

{#if serverActive}
  <Button
    variant="danger"
    class="button-icon-only"
    onclick={onStop}
    disabled={loading}
    title={loading ? 'Stopping...' : 'Stop Server'}
  >
    <MaterialIcon name="stop-circle" width="32" height="32" />
  </Button>
{:else}
  <Button
    variant="success"
    class="button-icon-only"
    onclick={onStart}
    disabled={loading || serverActive}
    title={loading
      ? 'Starting...'
      : serverActive
        ? 'Server is running'
        : 'Start Server'}
  >
    <MaterialIcon name="play" width="32" height="32" />
  </Button>
{/if}

