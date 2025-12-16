<script lang="ts">
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'

  export const showConfig: boolean = false
  export const showLlamaConfig: boolean = false
  export let showTerminal: boolean = false
  export let onToggleConfig: () => void
  export let onToggleLlamaConfig: () => void
  export let onToggleTerminal: () => void
</script>

<PageSubHeader title="AI Agent">
  {#snippet actions()}
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
  {/snippet}
</PageSubHeader>
