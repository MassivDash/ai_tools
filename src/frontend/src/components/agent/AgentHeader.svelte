<script lang="ts">
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'

  export const showConfig: boolean = false
  export const showLlamaConfig: boolean = false
  export let showTerminal: boolean = false
  export let onToggleConfig: () => void
  export let onToggleLlamaConfig: () => void
  export let onToggleTerminal: () => void
</script>

<div class="chat-header">
  <h3>AI Agent</h3>
  <div class="header-actions">
    <Button
      variant="info"
      class="button-icon-only"
      onclick={onToggleConfig}
      title="Agent Config"
    >
      <MaterialIcon name="robot-confused" width="32" height="32" />
    </Button>
    <Button
      variant="info"
      class="button-icon-only"
      onclick={onToggleLlamaConfig}
      title="Llama Server Config"
    >
      <MaterialIcon name="server-network" width="32" height="32" />
    </Button>
    <Button
      variant="info"
      class="button-icon-only"
      onclick={onToggleTerminal}
      title={showTerminal ? 'Hide Terminal' : 'Show Terminal'}
    >
      <MaterialIcon name="console" width="32" height="32" />
    </Button>
    <slot />
  </div>
</div>

<style>
  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    transition: border-color 0.3s ease;
  }

  .chat-header h3 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    font-size: 1.5rem;
    transition: color 0.3s ease;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .header-actions :global(.button-icon-only) {
    padding: 0.75rem !important;
    min-width: 3rem !important;
    min-height: 3rem !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .header-actions :global(.button-icon-only) :global(svg) {
    flex-shrink: 0;
  }
</style>
