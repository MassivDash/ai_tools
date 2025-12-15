<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import { marked } from 'marked'
  import { useAgentWebSocket } from '../../hooks/useAgentWebSocket'

  // Configure marked and WebSocket
  onMount(() => {
    try {
      if (marked && typeof marked.setOptions === 'function') {
        marked.setOptions({
          breaks: true,
          gfm: true
        })
      }
    } catch (e) {
      console.error('Failed to configure marked:', e)
    }
    // Connect to agent WebSocket for real-time updates
    agentWs.connect()
  })

  onDestroy(() => {
    // Disconnect WebSocket when component is destroyed
    agentWs.disconnect()
  })

  interface AgentChatRequest {
    message: string
    conversation_id?: string
  }

  interface AgentStreamEvent {
    type:
      | 'status'
      | 'tool_call'
      | 'tool_result'
      | 'text_chunk'
      | 'done'
      | 'error'
    status?: string
    message?: string
    tool_name?: string
    arguments?: string
    success?: boolean
    result?: string
    text?: string
    conversation_id?: string
    tool_calls?: Array<{
      tool_name: string
      result: string
    }>
  }

  interface ChatMessage {
    id: string // Unique ID for each message
    role: 'user' | 'assistant' | 'status' | 'tool'
    content: string
    timestamp: number
    toolName?: string
    statusType?: string
  }

  let messages: ChatMessage[] = []
  let inputMessage = ''
  let loading = false
  let error = ''
  let conversationId: string | null = null
  let chatContainer: HTMLDivElement
  let currentStreamingMessage: string = ''
  let streamingMessageId: string | null = null

  // Generate unique ID for messages
  const generateMessageId = () => {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
  }

  // WebSocket for real-time agent updates
  const agentWs = useAgentWebSocket(
    (event) => {
      handleStreamEvent(event)
    },
    (err) => {
      console.error('❌ Agent WebSocket error:', err)
    }
  )

  const sendMessage = async () => {
    if (!inputMessage.trim() || loading) return

    const userMessage: ChatMessage = {
      id: generateMessageId(),
      role: 'user',
      content: inputMessage.trim(),
      timestamp: Date.now()
    }

    messages = [...messages, userMessage]
    const currentInput = inputMessage.trim()
    inputMessage = ''
    loading = true
    error = ''
    currentStreamingMessage = ''
    streamingMessageId = null

    // Scroll to bottom
    setTimeout(() => {
      if (chatContainer) {
        chatContainer.scrollTop = chatContainer.scrollHeight
      }
    }, 100)

    try {
      const request: AgentChatRequest = {
        message: currentInput,
        conversation_id: conversationId || undefined
      }

      // Send message via WebSocket - events will come back via WebSocket
      const baseUrl = axiosBackendInstance.defaults.baseURL || ''
      const response = await window.fetch(`${baseUrl}/agent/chat/stream`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(request)
      })

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`)
      }

      // Events will come via WebSocket only (no SSE reading to avoid duplicates)
      // Just wait for the response to complete - WebSocket handles all events

      // Scroll to bottom
      setTimeout(() => {
        if (chatContainer) {
          chatContainer.scrollTop = chatContainer.scrollHeight
        }
      }, 100)
    } catch (err: any) {
      console.error('❌ Failed to send message:', err)
      error =
        err.response?.data?.error ||
        err.response?.data?.message ||
        err.message ||
        'Failed to send message'
    } finally {
      loading = false
    }
  }

  const handleStreamEvent = (event: AgentStreamEvent) => {
    switch (event.type) {
      case 'status':
        if (event.message) {
          // Always show status messages - they indicate what the agent is doing
          // Remove any existing status message and add new one
          messages = messages.filter((m) => m.role !== 'status')
          messages.push({
            id: generateMessageId(),
            role: 'status',
            content: event.message,
            timestamp: Date.now(),
            statusType: event.status
          })
          // Auto-scroll when status updates
          setTimeout(() => {
            if (chatContainer) {
              chatContainer.scrollTop = chatContainer.scrollHeight
            }
          }, 10)
        }
        break

      case 'tool_call':
        // Remove any existing tool message for this tool
        messages = messages.filter(
          (m) => m.role !== 'tool' || m.toolName !== event.tool_name
        )
        messages.push({
          id: generateMessageId(),
          role: 'tool',
          content: `Calling ${event.tool_name}...`,
          timestamp: Date.now(),
          toolName: event.tool_name
        })
        break

      case 'tool_result': {
        // Remove status message when tool completes
        messages = messages.filter((m) => m.role !== 'status')
        // Update existing tool message or create new one
        const toolIndex = messages.findIndex(
          (m) => m.role === 'tool' && m.toolName === event.tool_name
        )
        if (toolIndex >= 0) {
          messages[toolIndex].content = event.success
            ? `✅ ${event.tool_name} completed`
            : `❌ ${event.tool_name} failed: ${event.result || 'Unknown error'}`
        } else {
          messages.push({
            id: generateMessageId(),
            role: 'tool',
            content: event.success
              ? `✅ ${event.tool_name} completed`
              : `❌ ${event.tool_name} failed: ${event.result || 'Unknown error'}`,
            timestamp: Date.now(),
            toolName: event.tool_name
          })
        }
        break
      }

      case 'text_chunk':
        if (event.text) {
          // Remove status message when text starts streaming
          messages = messages.filter((m) => m.role !== 'status')

          currentStreamingMessage += event.text
          // Update or create streaming message
          if (streamingMessageId) {
            const existingIndex = messages.findIndex(
              (m) => m.id === streamingMessageId
            )
            if (existingIndex >= 0) {
              messages[existingIndex].content = currentStreamingMessage
            } else {
              // Message was removed somehow, create new one
              streamingMessageId = generateMessageId()
              messages.push({
                id: streamingMessageId,
                role: 'assistant',
                content: currentStreamingMessage,
                timestamp: 0 // Mark as streaming
              })
            }
          } else {
            // Create new streaming message
            streamingMessageId = generateMessageId()
            messages.push({
              id: streamingMessageId,
              role: 'assistant',
              content: currentStreamingMessage,
              timestamp: 0 // Mark as streaming
            })
          }
          // Auto-scroll during streaming
          setTimeout(() => {
            if (chatContainer) {
              chatContainer.scrollTop = chatContainer.scrollHeight
            }
          }, 10)
        }
        break

      case 'done':
        if (event.conversation_id) {
          conversationId = event.conversation_id
        }
        // Remove any remaining status messages
        messages = messages.filter((m) => m.role !== 'status')
        // Mark streaming message as complete
        if (streamingMessageId) {
          const streamingIndex = messages.findIndex(
            (m) => m.id === streamingMessageId
          )
          if (streamingIndex >= 0) {
            messages[streamingIndex].timestamp = Date.now()
          }
          streamingMessageId = null
        }
        currentStreamingMessage = ''
        break

      case 'error':
        error = event.message || 'An error occurred'
        // Clear streaming message on error
        if (streamingMessageId) {
          messages = messages.filter((m) => m.id !== streamingMessageId)
          streamingMessageId = null
        }
        currentStreamingMessage = ''
        break
    }
  }

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      sendMessage()
    }
  }

  const clearChat = () => {
    messages = []
    conversationId = null
    error = ''
  }

  const renderMarkdown = (content: string): string => {
    try {
      if (marked && typeof marked.parse === 'function') {
        return marked.parse(content) as string
      }
      // Fallback: simple markdown-like formatting if marked is not available
      return content
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/\n/g, '<br>')
        .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
        .replace(/\*(.*?)\*/g, '<em>$1</em>')
        .replace(/`(.*?)`/g, '<code>$1</code>')
    } catch (e) {
      console.error('Failed to render markdown:', e)
      return content
    }
  }
</script>

<div class="chat-interface">
  <div class="chat-header-bar">
    <h4>Chat</h4>
    {#if messages.length > 0}
      <Button variant="secondary" onclick={clearChat} class="clear-button">
        Clear Chat
      </Button>
    {/if}
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="chat-messages" bind:this={chatContainer}>
    {#if messages.length === 0}
      <div class="empty-chat">
        <p>👋 Start a conversation with the AI agent</p>
        <p class="hint">
          Ask questions and the agent will use its tools to help you
        </p>
      </div>
    {:else}
      {#each messages as message (message.id)}
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
            class:streaming={message.role === 'assistant' &&
              message.timestamp === 0}
          >
            <div class="message-role">
              {message.role === 'user' ? 'You' : 'Assistant'}
            </div>
            <div
              class="message-content"
              class:markdown={message.role === 'assistant' &&
                message.timestamp !== 0}
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
      {/each}
      {#if loading}
        <div class="message assistant">
          <div class="message-role">Assistant</div>
          <div class="message-content loading">
            <span class="typing-indicator">
              <span></span>
              <span></span>
              <span></span>
            </span>
          </div>
        </div>
      {/if}
    {/if}
  </div>

  <div class="chat-input-container">
    <textarea
      bind:value={inputMessage}
      onkeypress={handleKeyPress}
      placeholder="Type your message... (Press Enter to send, Shift+Enter for new line)"
      disabled={loading}
      class="chat-input"
      rows="3"
    ></textarea>
    <Button
      variant="primary"
      onclick={sendMessage}
      disabled={loading || !inputMessage.trim()}
      class="send-button"
    >
      {loading ? 'Sending...' : 'Send'}
    </Button>
  </div>
</div>

<style>
  .chat-interface {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 80vh;
    background-color: var(--bg-primary, #fff);
  }

  .chat-header-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
  }

  .chat-header-bar h4 {
    margin: 0;
    color: var(--text-primary, #100f0f);
  }

  .clear-button {
    font-size: 0.9rem;
    padding: 0.5rem 1rem;
  }

  .error {
    padding: 0.75rem 1rem;
    background-color: rgba(255, 200, 200, 0.2);
    border-bottom: 1px solid rgba(255, 100, 100, 0.5);
    color: var(--accent-color, #c33);
    font-size: 0.9rem;
  }

  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .empty-chat {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary, #666);
    text-align: center;
  }

  .empty-chat .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary, #999);
    margin-top: 0.5rem;
  }

  .message {
    display: flex;
    flex-direction: column;
    max-width: 80%;
    animation: fadeIn 0.3s ease-in;
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

  .message-content.markdown {
    /* Markdown styling */
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

  .message-content.loading {
    background-color: transparent;
    border: none;
    padding: 0.5rem;
  }

  .typing-indicator {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .typing-indicator span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--text-secondary, #999);
    animation: typing 1.4s infinite;
  }

  .typing-indicator span:nth-child(2) {
    animation-delay: 0.2s;
  }

  .typing-indicator span:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes typing {
    0%,
    60%,
    100% {
      transform: translateY(0);
      opacity: 0.7;
    }
    30% {
      transform: translateY(-10px);
      opacity: 1;
    }
  }

  .chat-input-container {
    display: flex;
    gap: 0.5rem;
    padding: 1rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
  }

  .chat-input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    font-family: inherit;
    font-size: 1rem;
    resize: vertical;
    min-height: 3rem;
    max-height: 10rem;
  }

  .chat-input:focus {
    outline: none;
    border-color: var(--accent-color, #2196f3);
  }

  .send-button {
    align-self: flex-end;
    padding: 0.75rem 1.5rem;
  }
</style>
