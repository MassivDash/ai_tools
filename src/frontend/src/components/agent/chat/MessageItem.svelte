<script lang="ts">
  import type { ChatMessage } from '../types'
  import { renderMarkdown } from '../utils/markdown'

  export let message: ChatMessage
</script>

{#if message.role === 'status'}
  <div class="message status-message">
    <div class="status-indicator">
      {#if message.statusType === 'thinking'}
        <span class="thinking-dots">
          <span></span>
          <span></span>
          <span></span>
        </span>
        Thinking...
      {:else if message.statusType === 'calling_tool'}
        🔧 {message.content}
      {:else if message.statusType === 'tool_executing'}
        ⚙️ {message.content}
      {:else if message.statusType === 'tool_complete'}
        ✅ {message.content}
      {:else if message.statusType === 'tool_error'}
        ❌ {message.content}
      {:else}
        {message.content}
      {/if}
    </div>
  </div>
{:else if message.role === 'tool'}
  <div class="message tool-message">
    <div class="tool-indicator">
      {message.content}
    </div>
  </div>
{:else}
  <div
    class="message"
    class:user={message.role === 'user'}
    class:assistant={message.role === 'assistant'}
    class:streaming={message.role === 'assistant' && message.timestamp === 0}
  >
    <div class="message-role">
      {message.role === 'user' ? 'You' : 'Assistant'}
    </div>
    <div
      class="message-content"
      class:markdown={message.role === 'assistant' && message.timestamp !== 0}
    >
      {#if message.role === 'assistant' && message.timestamp === 0}
        {@html renderMarkdown(message.content)}
        <span class="streaming-cursor">|</span>
      {:else}
        {@html renderMarkdown(message.content)}
      {/if}
    </div>
  </div>
{/if}

<style>
  .message {
    display: flex;
    flex-direction: column;
    max-width: 80%;
    animation: fadeIn 0.3s ease-in;
    padding: 0 1rem;
  }

  .message.user {
    align-self: flex-end;
  }

  .message.assistant {
    align-self: flex-start;
  }

  .message.streaming {
    opacity: 0.95;
  }

  .status-message {
    align-self: center;
    max-width: 100%;
    margin: 0.5rem 0;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background-color: var(--bg-tertiary, #f0f0f0);
    border-radius: 20px;
    font-size: 0.85rem;
    color: var(--text-secondary, #666);
    border: 1px solid var(--border-color, #e0e0e0);
  }

  .thinking-dots {
    display: inline-flex;
    gap: 0.2rem;
    align-items: center;
  }

  .thinking-dots span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--text-secondary, #666);
    animation: thinking 1.4s infinite;
  }

  .thinking-dots span:nth-child(2) {
    animation-delay: 0.2s;
  }

  .thinking-dots span:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes thinking {
    0%,
    60%,
    100% {
      transform: translateY(0);
      opacity: 0.7;
    }
    30% {
      transform: translateY(-8px);
      opacity: 1;
    }
  }

  .tool-message {
    align-self: center;
    max-width: 100%;
    margin: 0.5rem 0;
  }

  .tool-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border-radius: 20px;
    font-size: 0.85rem;
    color: var(--text-primary, #100f0f);
    border: 1px solid var(--border-color, #e0e0e0);
  }

  .streaming-cursor {
    display: inline-block;
    animation: blink 1s infinite;
    color: var(--accent-color, #2196f3);
    font-weight: bold;
  }

  @keyframes blink {
    0%,
    50% {
      opacity: 1;
    }
    51%,
    100% {
      opacity: 0;
    }
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .message-role {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary, #666);
    margin-bottom: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .message-content {
    padding: 0.75rem 1rem;
    border-radius: 8px;
    line-height: 1.6;
    word-wrap: break-word;
  }

  .message.user .message-content {
    background-color: var(--accent-color, #2196f3);
    color: white;
  }

  .message.assistant .message-content {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--text-primary, #100f0f);
    border: 1px solid var(--border-color, #e0e0e0);
  }

  .message-content.markdown :global(h1),
  .message-content.markdown :global(h2),
  .message-content.markdown :global(h3),
  .message-content.markdown :global(h4),
  .message-content.markdown :global(h5),
  .message-content.markdown :global(h6) {
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #100f0f);
  }

  .message-content.markdown :global(h1) {
    font-size: 1.5rem;
  }

  .message-content.markdown :global(h2) {
    font-size: 1.3rem;
  }

  .message-content.markdown :global(h3) {
    font-size: 1.1rem;
  }

  .message-content.markdown :global(p) {
    margin: 0.5rem 0;
  }

  .message-content.markdown :global(ul),
  .message-content.markdown :global(ol) {
    margin: 0.5rem 0;
    padding-left: 1.5rem;
  }

  .message-content.markdown :global(li) {
    margin: 0.25rem 0;
  }

  .message-content.markdown :global(code) {
    background-color: rgba(0, 0, 0, 0.1);
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.9em;
  }

  .message-content.markdown :global(pre) {
    background-color: rgba(0, 0, 0, 0.05);
    padding: 0.75rem;
    border-radius: 4px;
    overflow-x: auto;
    margin: 0.5rem 0;
  }

  .message-content.markdown :global(pre code) {
    background-color: transparent;
    padding: 0;
  }

  .message-content.markdown :global(blockquote) {
    border-left: 3px solid var(--accent-color, #2196f3);
    padding-left: 1rem;
    margin: 0.5rem 0;
    color: var(--text-secondary, #666);
    font-style: italic;
  }

  .message-content.markdown :global(a) {
    color: var(--accent-color, #2196f3);
    text-decoration: underline;
  }
</style>
