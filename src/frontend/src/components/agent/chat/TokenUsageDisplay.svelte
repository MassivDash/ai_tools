<script lang="ts">
  import MaterialIcon from '../../ui/MaterialIcon.svelte'

  interface Props {
    tokenUsage: {
      prompt_tokens: number
      completion_tokens: number
      total_tokens: number
    }
    ctxSize: number
  }

  let { tokenUsage, ctxSize }: Props = $props()
</script>

<div class="token-display">
  <MaterialIcon name="memory" width="16" height="16" />
  <span class="usage-text">
    {#if ctxSize > 0}
      {tokenUsage.total_tokens} / {ctxSize} tokens ({Math.round(
        (tokenUsage.total_tokens / ctxSize) * 100
      )}%)
    {:else}
      {tokenUsage.total_tokens} tokens
    {/if}
  </span>
</div>

<style>
  .token-display {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem 1.5rem;
    font-size: 0.8rem;
    color: var(--text-secondary, #666);
    background-color: var(--bg-secondary, #f9f9f9);
    border-top: 1px solid var(--border-color, #e0e0e0);
  }

  .usage-text {
    font-family: 'JetBrains Mono', monospace;
  }

  @media screen and (max-width: 768px) {
    .token-display {
      padding: 0.5rem 1rem;
      font-size: 0.75rem;
    }
  }
</style>
