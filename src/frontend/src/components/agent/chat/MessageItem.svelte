<script lang="ts">
  import type { ChatMessage } from '../types'
  import { renderMarkdown } from '../utils/markdown'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'

  export let message: ChatMessage

  // Helper to get tool icon based on tool name
  const getToolIcon = (toolName: string | undefined): string => {
    if (!toolName) return 'wrench'
    const name = toolName.toLowerCase()
    if (name.includes('chromadb') || name.includes('chroma')) return 'database'
    if (name.includes('financial')) return 'currency-usd'
    return 'wrench'
  }

  // Helper to get file icon based on attachment type
  const getFileIcon = (type: string): string => {
    switch (type) {
      case 'text':
        return 'note-text'
      case 'pdf':
        return 'file-pdf-box'
      case 'image':
        return 'image'
      case 'audio':
        return 'microphone'
      default:
        return 'file'
    }
  }

  // Helper to determine if tool message is success or error
  const isToolSuccess = (content: string): boolean => {
    return content.includes('✅') || content.includes('completed')
  }

  const isToolError = (content: string): boolean => {
    return content.includes('❌') || content.includes('failed')
  }
</script>

{#if message.role === 'status'}
  <div class="message status-message">
    <div class="status-indicator">
      {#if message.statusType === 'thinking'}
        <div class="spinning-cog">
          <MaterialIcon name="cog" width="16" height="16" />
        </div>
        <span>Thinking...</span>
      {:else if message.statusType === 'calling_tool'}
        <MaterialIcon name="wrench" width="16" height="16" />
        <span>{message.content}</span>
      {:else if message.statusType === 'tool_executing'}
        <div class="spinning-cog">
          <MaterialIcon name="cog" width="16" height="16" />
        </div>
        <span>{message.content}</span>
      {:else if message.statusType === 'tool_complete'}
        <MaterialIcon
          name="check-circle"
          width="16"
          height="16"
          class="success-icon"
        />
        <span>{message.content}</span>
      {:else if message.statusType === 'tool_error'}
        <MaterialIcon
          name="close-circle"
          width="16"
          height="16"
          class="error-icon"
        />
        <span>{message.content}</span>
      {:else}
        <span>{message.content}</span>
      {/if}
    </div>
  </div>
{:else if message.role === 'tool'}
  <div class="message tool-message">
    <div
      class="tool-indicator"
      class:success={isToolSuccess(message.content)}
      class:error={isToolError(message.content)}
    >
      <MaterialIcon
        name={getToolIcon(message.toolName)}
        width="18"
        height="18"
        class="tool-icon"
      />
      <span class="tool-text"
        >{message.content.replace(/✅|❌/g, '').trim()}</span
      >
      {#if isToolSuccess(message.content)}
        <MaterialIcon
          name="check-circle"
          width="16"
          height="16"
          class="status-icon success-icon"
        />
      {:else if isToolError(message.content)}
        <MaterialIcon
          name="close-circle"
          width="16"
          height="16"
          class="status-icon error-icon"
        />
      {/if}
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
      {#if message.role === 'assistant'}
        <MaterialIcon name="robot" width="14" height="14" class="role-icon" />
      {/if}
      <span>{message.role === 'user' ? 'You' : 'Assistant'}</span>
    </div>
    <div
      class="message-content"
      class:markdown={message.role === 'assistant' && message.timestamp !== 0}
    >
      {#if message.attachments && message.attachments.length > 0}
        <div class="attachments-display">
          {#each message.attachments as attachment (attachment.name)}
            <div class="attachment-icon">
              <MaterialIcon
                name={getFileIcon(attachment.type)}
                width="20"
                height="20"
              />
              <span class="attachment-label">{attachment.name}</span>
            </div>
          {/each}
        </div>
      {/if}
      {#if message.role === 'assistant' && message.timestamp === 0}
        {@html renderMarkdown(message.content)}
        <span class="typing-indicator-inline">
          <span></span>
          <span></span>
          <span></span>
        </span>
      {:else if message.content && message.content !== 'Sent files'}
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

  .spinning-cog {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-color, #2196f3);
    animation: spin 2s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .success-icon {
    color: #4caf50;
  }

  .error-icon {
    color: #f44336;
  }

  .tool-message {
    align-self: center;
    max-width: 100%;
    margin: 0.5rem 0;
  }

  .tool-indicator {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border-radius: 8px;
    font-size: 0.875rem;
    color: var(--text-primary, #100f0f);
    border: 1px solid var(--border-color, #e0e0e0);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    transition: all 0.2s ease;
  }

  .tool-indicator.success {
    background-color: rgba(76, 175, 80, 0.1);
    border-color: rgba(76, 175, 80, 0.3);
  }

  .tool-indicator.error {
    background-color: rgba(244, 67, 54, 0.1);
    border-color: rgba(244, 67, 54, 0.3);
  }

  .tool-icon {
    color: var(--accent-color, #2196f3);
    flex-shrink: 0;
  }

  .tool-text {
    flex: 1;
    font-weight: 500;
  }

  .status-icon {
    flex-shrink: 0;
  }

  .typing-indicator-inline {
    display: inline-flex;
    gap: 0.25rem;
    align-items: center;
    margin-left: 0.5rem;
    vertical-align: middle;
  }

  .typing-indicator-inline span {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background-color: var(--accent-color, #2196f3);
    animation: typing-dot 1.4s infinite;
    display: inline-block;
  }

  .typing-indicator-inline span:nth-child(2) {
    animation-delay: 0.2s;
  }

  .typing-indicator-inline span:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes typing-dot {
    0%,
    60%,
    100% {
      transform: translateY(0);
      opacity: 0.7;
    }
    30% {
      transform: translateY(-4px);
      opacity: 1;
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
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary, #666);
    margin-bottom: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .role-icon {
    color: var(--accent-color, #2196f3);
    flex-shrink: 0;
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

  .attachments-display {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-color, #e0e0e0);
  }

  .attachment-icon {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-tertiary, #f0f0f0);
    border-radius: 8px;
    font-size: 0.875rem;
    color: var(--text-primary, #100f0f);
  }

  .message.user .attachment-icon {
    background-color: rgba(255, 255, 255, 0.2);
    color: white;
  }

  .attachment-label {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
