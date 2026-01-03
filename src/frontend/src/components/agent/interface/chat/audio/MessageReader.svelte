<script lang="ts">
  import Button from '@ui/Button.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'

  interface Props {
    enabled?: boolean
    speaking?: boolean
    onToggle: () => void
    onStop: () => void
    lang?: string
  }

  let {
    enabled = false,
    speaking = false,
    onToggle,
    onStop,
    lang = 'en-US'
  }: Props = $props()
</script>

{#if onToggle}
  <Button
    variant="ghost"
    class="voice-input-button {enabled || speaking ? 'speaking' : ''}"
    onclick={() => {
      if (speaking && onStop) {
        onStop()
      } else if (onToggle) {
        onToggle()
      }
    }}
    title={speaking
      ? 'Stop Speaking'
      : enabled
        ? 'Read Messages: On'
        : 'Read Messages: Off'}
  >
    <MaterialIcon
      name={speaking ? 'stop' : enabled ? 'volume-high' : 'volume-off'}
      width="16"
      height="16"
    />
    <span class="label">
      {speaking
        ? 'Speaking'
        : enabled
          ? 'Read Messages: On'
          : 'Read Messages: Off'}
    </span>
  </Button>
{/if}

<style>
  :global(.voice-input-button) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 0.9rem;
    justify-content: flex-start;
    min-width: 10rem;
  }

  :global(.voice-input-button:active) {
    color: var(--accent-color, #2196f3);
    background-color: rgba(33, 150, 243, 0.1);
  }

  :global(.voice-input-button.speaking) {
    color: var(--accent-color, #2196f3);
    background-color: rgba(33, 150, 243, 0.1);
  }

  :global(.voice-input-button:hover:not(:disabled)) {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--accent-color, #2196f3);
  }

  .label {
    line-height: 1;
    font-weight: 500;
  }
</style>
