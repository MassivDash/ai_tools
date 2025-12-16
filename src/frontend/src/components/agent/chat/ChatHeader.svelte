<script lang="ts">
  import Button from '../../ui/Button.svelte'
  import Badge from '../../ui/Badge.svelte'
  import { formatToolName } from '../utils/formatting'

  export let activeToolsList: string[] = []
  export let hasMessages: boolean = false
  export let onClear: () => void
</script>

<div class="chat-header-bar">
  <div class="header-left">
    <h4>Chat</h4>
    {#if activeToolsList.length > 0}
      <div class="tools-section">
        <span class="tools-label">Tools:</span>
        <div class="tools-badges">
          {#each activeToolsList as tool}
            <Badge variant="primary">{formatToolName(tool)}</Badge>
          {/each}
        </div>
      </div>
    {/if}
  </div>
  {#if hasMessages}
    <Button variant="secondary" onclick={onClear} class="clear-button">
      Clear Chat
    </Button>
  {/if}
</div>

<style>
  .chat-header-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background-color: var(--bg-secondary, #f9f9f9);
    gap: 1rem;
    transition: background-color 0.3s ease;
  }

  .header-left {
    display: flex;
    align-items: center;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
  }

  .chat-header-bar h4 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    white-space: nowrap;
  }

  .tools-section {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .tools-label {
    font-size: 0.875rem;
    color: var(--text-secondary, #666);
    font-weight: 500;
    white-space: nowrap;
  }

  .tools-badges {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .clear-button {
    font-size: 0.9rem;
    padding: 0.5rem 1rem;
  }

  @media screen and (max-width: 768px) {
    .tools-section {
      width: 100%;
    }
  }
</style>
