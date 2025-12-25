<script lang="ts">
  import Chart from '../../ui/Chart.svelte'
  import type { ChatMessage } from '../types'
  import { renderMarkdown } from '../utils/markdown'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import { getToolIconFromMetadata, getToolIcon } from '../utils/toolIcons'
  import { icons } from '@iconify-json/mdi'

  interface Props {
    message: ChatMessage
  }

  let { message }: Props = $props()

  // Track tool icon for this message
  let toolIcon: string = $state('wrench')

  // Reference to message content container
  let messageContentElement: HTMLDivElement = $state()

  // Load tool icon when component mounts or tool name changes
  $effect(() => {
    if (message.toolName) {
      // Set a fallback immediately using pattern matching
      toolIcon = getToolIcon(message.toolName)
      // Then try to get icon from metadata (will update if different)
      getToolIconFromMetadata(message.toolName).then((icon) => {
        toolIcon = icon
      })
    }
  })

  // Add copy buttons to code blocks only after message is complete (not during streaming)
  $effect(() => {
    // Only run for completed messages (timestamp !== 0 means message is done)
    // Skip streaming messages (timestamp === 0) to avoid interfering with rendering
    const isComplete =
      message.role === 'assistant' ? message.timestamp !== 0 : true
    const isUserOrAssistant =
      message.role === 'assistant' || message.role === 'user'

    if (
      messageContentElement &&
      isUserOrAssistant &&
      message.content &&
      isComplete
    ) {
      // Wait a bit after message is complete to ensure all markdown is fully rendered
      const timeoutId = setTimeout(() => {
        addCopyButtonsToCodeBlocks(messageContentElement)
      }, 200)

      return () => {
        clearTimeout(timeoutId)
      }
    }
  })

  const addCopyButtonsToCodeBlocks = (container: HTMLElement) => {
    // Remove existing copy buttons first to avoid duplicates
    const existingButtons = container.querySelectorAll('.code-copy-button')
    existingButtons.forEach((btn) => btn.remove())

    // Remove has-copy-button class from all pre elements
    const allPres = container.querySelectorAll('pre.has-copy-button')
    allPres.forEach((pre) => pre.classList.remove('has-copy-button'))

    // Find all pre elements (code blocks)
    const preElements = container.querySelectorAll('pre')

    preElements.forEach((pre) => {
      if (pre.classList.contains('has-copy-button')) {
        return // Already processed
      }
      pre.classList.add('has-copy-button')

      // Get the code element inside
      const codeElement = pre.querySelector('code')
      if (!codeElement) {
        return // No code element found
      }

      // Create copy button with Material Icon
      const copyButton = document.createElement('button')
      copyButton.className = 'code-copy-button'
      copyButton.setAttribute('aria-label', 'Copy code')

      // Get Material Design icon data for copy icon
      const copyIconData = icons.icons['content-copy']
      const iconViewBox = `0 0 ${icons.width || 24} ${icons.height || 24}`

      if (copyIconData) {
        copyButton.innerHTML = `
          <svg width="16" height="16" viewBox="${iconViewBox}" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
            ${copyIconData.body}
          </svg>
        `
      } else {
        // Fallback if icon not found
        copyButton.innerHTML = `
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
            <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
          </svg>
        `
      }

      // Position button
      pre.style.position = 'relative'
      copyButton.style.position = 'absolute'
      copyButton.style.top = '0.5rem'
      copyButton.style.right = '0.5rem'
      copyButton.style.padding = '0.375rem'
      copyButton.style.background = 'var(--bg-primary, #fff)'
      copyButton.style.border = '1px solid var(--border-color, #e0e0e0)'
      copyButton.style.borderRadius = '4px'
      copyButton.style.cursor = 'pointer'
      copyButton.style.display = 'flex'
      copyButton.style.alignItems = 'center'
      copyButton.style.justifyContent = 'center'
      copyButton.style.opacity = '0.7'
      copyButton.style.transition = 'opacity 0.2s, color 0.2s'
      copyButton.style.zIndex = '10'
      copyButton.style.color = 'var(--text-primary, #100f0f)'

      // Hover effect
      copyButton.addEventListener('mouseenter', () => {
        copyButton.style.opacity = '1'
      })
      copyButton.addEventListener('mouseleave', () => {
        copyButton.style.opacity = '0.7'
      })

      // Copy functionality
      copyButton.addEventListener('click', async () => {
        const textToCopy =
          codeElement.textContent || codeElement.innerText || ''
        try {
          await window.navigator.clipboard.writeText(textToCopy)
          // Visual feedback - show checkmark icon
          const checkIconData = icons.icons['check']
          const iconViewBox = `0 0 ${icons.width || 24} ${icons.height || 24}`
          const originalHTML = copyButton.innerHTML

          if (checkIconData) {
            copyButton.innerHTML = `
              <svg width="16" height="16" viewBox="${iconViewBox}" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                ${checkIconData.body}
              </svg>
            `
          } else {
            // Fallback checkmark
            copyButton.innerHTML = `
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
              </svg>
            `
          }

          copyButton.style.color = 'var(--success-color, #4caf50)'
          setTimeout(() => {
            copyButton.innerHTML = originalHTML
            copyButton.style.color = 'var(--text-primary, #100f0f)'
          }, 2000)
        } catch (err) {
          console.error('Failed to copy code:', err)
        }
      })

      pre.appendChild(copyButton)
    })
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
  const isToolSuccess = (content: string | any[]): boolean => {
    if (typeof content !== 'string') return false
    return content.includes('✅') || content.includes('completed')
  }

  const isToolError = (content: string | any[]): boolean => {
    if (typeof content !== 'string') return false
    return content.includes('❌') || content.includes('failed')
  }

  // Helper to parse content for charts
  const parseContentWithCharts = (content: string) => {
    if (typeof content !== 'string') return [{ type: 'text', content }]

    const parts = []
    const regex = /```json-chart\n([\s\S]*?)\n```/g
    let lastIndex = 0
    let match

    while ((match = regex.exec(content)) !== null) {
      // Add text before chart
      if (match.index > lastIndex) {
        parts.push({
          type: 'text',
          content: content.slice(lastIndex, match.index)
        })
      }

      // Add chart
      try {
        const chartData = JSON.parse(match[1])
        parts.push({
          type: 'chart',
          data: chartData
        })
      } catch (e) {
        console.error('Failed to parse chart data:', e)
        // Fallback: treat as normal text if parsing fails
        parts.push({
          type: 'text',
          content: match[0]
        })
      }

      lastIndex = regex.lastIndex
    }

    // Add remaining text
    if (lastIndex < content.length) {
      parts.push({
        type: 'text',
        content: content.slice(lastIndex)
      })
    }

    return parts
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
      <MaterialIcon name={toolIcon} width="18" height="18" class="tool-icon" />
      <span class="tool-text"
        >{(typeof message.content === 'string'
          ? message.content
          : 'Tool execution'
        )
          .replace(/✅|❌/g, '')
          .trim()}</span
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
      bind:this={messageContentElement}
    >
      {#if message.attachments && message.attachments.length > 0}
        <div class="attachments-display">
          {#each message.attachments.filter((a) => a.type !== 'image') as attachment (attachment.name)}
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
        {#if Array.isArray(message.content)}
          {#each message.content as part}
            {#if part.type === 'text'}
              {#each parseContentWithCharts(part.text) as subpart}
                {#if subpart.type === 'text'}
                  {@html renderMarkdown(subpart.content)}
                {:else if subpart.type === 'chart'}
                  <Chart data={subpart.data} />
                {/if}
              {/each}
            {:else if part.type === 'image_url'}
              <img
                src={part.image_url.url}
                alt="User upload"
                class="message-image"
              />
            {/if}
          {/each}
        {:else}
          {#each parseContentWithCharts(message.content) as part}
            {#if part.type === 'text'}
              {@html renderMarkdown(part.content)}
            {:else if part.type === 'chart'}
              <Chart data={part.data} />
            {/if}
          {/each}
        {/if}
        <span class="typing-indicator-inline">
          <span></span>
          <span></span>
          <span></span>
        </span>
      {:else if message.content && message.content !== 'Sent files'}
        {#if Array.isArray(message.content)}
          {#each message.content as part}
            {#if part.type === 'text'}
              {#each parseContentWithCharts(part.text) as subpart}
                {#if subpart.type === 'text'}
                  {@html renderMarkdown(subpart.content)}
                {:else if subpart.type === 'chart'}
                  <Chart data={subpart.data} />
                {/if}
              {/each}
            {:else if part.type === 'image_url'}
              <img
                src={part.image_url.url}
                alt="User upload"
                class="message-image"
              />
            {/if}
          {/each}
        {:else}
          {#each parseContentWithCharts(message.content) as part}
            {#if part.type === 'text'}
              {@html renderMarkdown(part.content)}
            {:else if part.type === 'chart'}
              <Chart data={part.data} />
            {/if}
          {/each}
        {/if}
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
    border-radius: 8px;
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

  .tool-text {
    flex: 1;
    font-weight: 500;
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
    border-radius: 8px;
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
    border-radius: 8px;
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.9em;
  }

  .message-content.markdown :global(pre) {
    background-color: rgba(0, 0, 0, 0.05);
    padding: 0.75rem;
    border-radius: 8px;
    overflow-x: auto;
    margin: 0.5rem 0;
    position: relative;
  }

  .message-content.markdown :global(pre:hover .code-copy-button) {
    opacity: 1;
  }

  .message-content.markdown :global(.code-copy-button) {
    color: var(--text-primary, #100f0f);
    background-color: var(--bg-primary, #fff);
    border-color: var(--border-color, #e0e0e0);
  }

  .message-content.markdown :global(.code-copy-button:hover) {
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--accent-color, #2196f3);
  }

  .message-content.markdown :global(.code-copy-button:active) {
    transform: scale(0.95);
  }

  /* Dark theme support */
  @media (prefers-color-scheme: dark) {
    .message-content.markdown :global(.code-copy-button) {
      color: var(--text-primary, #e0e0e0);
      background-color: var(--bg-primary, #1e1e1e);
      border-color: var(--border-color, #404040);
    }

    .message-content.markdown :global(.code-copy-button:hover) {
      background-color: var(--bg-secondary, #2a2a2a);
      color: var(--accent-color, #64b5f6);
    }
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

  .message-content.markdown :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
    font-size: 0.9em;
    max-width: 100%;
    display: table;
    table-layout: auto;
  }

  /* Wrap table in scrollable container for mobile */
  .message-content.markdown {
    overflow-x: auto;
  }

  .message-content.markdown :global(th),
  .message-content.markdown :global(td) {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border: 1px solid var(--border-color, #e0e0e0);
    word-wrap: break-word;
  }

  .message-content.markdown :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 8px;
    margin: 0.5rem 0;
    display: block;
  }

  .message-content.markdown :global(th) {
    background-color: var(--bg-tertiary, #f0f0f0);
    font-weight: 600;
    color: var(--text-primary, #100f0f);
  }

  .message-content.markdown :global(td) {
    background-color: var(--bg-primary, #fff);
    color: var(--text-primary, #100f0f);
  }

  .message-content.markdown :global(tr:nth-child(even) td) {
    background-color: var(--bg-secondary, #f5f5f5);
  }

  .message-content.markdown :global(tr:hover td) {
    background-color: var(--bg-tertiary, #f0f0f0);
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
  .message-image {
    max-width: 100%;
    height: auto;
    border-radius: 8px;
    margin: 0.5rem 0;
    display: block;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }
</style>
