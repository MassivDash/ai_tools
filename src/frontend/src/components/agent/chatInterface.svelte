<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import { marked } from 'marked'

  // Configure marked
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
  })

  interface ChatMessage {
    role: 'user' | 'assistant'
    content: string
    timestamp: number
  }

  interface AgentChatRequest {
    message: string
    conversation_id?: string
  }

  interface AgentChatResponse {
    success: boolean
    message: string
    conversation_id?: string
    tool_calls?: Array<{
      tool_name: string
      result: string
    }>
  }

  let messages: ChatMessage[] = []
  let inputMessage = ''
  let loading = false
  let error = ''
  let conversationId: string | null = null
  let chatContainer: HTMLDivElement

  const sendMessage = async () => {
    if (!inputMessage.trim() || loading) return

    const userMessage: ChatMessage = {
      role: 'user',
      content: inputMessage.trim(),
      timestamp: Date.now()
    }

    messages = [...messages, userMessage]
    const currentInput = inputMessage.trim()
    inputMessage = ''
    loading = true
    error = ''

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

      const response = await axiosBackendInstance.post<AgentChatResponse>(
        'agent/chat',
        request
      )

      if (response.data.success) {
        if (response.data.conversation_id) {
          conversationId = response.data.conversation_id
        }

        const assistantMessage: ChatMessage = {
          role: 'assistant',
          content: response.data.message,
          timestamp: Date.now()
        }

        messages = [...messages, assistantMessage]

        // Scroll to bottom after message is added
        setTimeout(() => {
          if (chatContainer) {
            chatContainer.scrollTop = chatContainer.scrollHeight
          }
        }, 100)
      } else {
        error = response.data.message || 'Failed to get response'
      }
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
      {#each messages as message (message.timestamp)}
        <div
          class="message"
          class:user={message.role === 'user'}
          class:assistant={message.role === 'assistant'}
        >
          <div class="message-role">
            {message.role === 'user' ? 'You' : 'Assistant'}
          </div>
          <div
            class="message-content"
            class:markdown={message.role === 'assistant'}
          >
            {@html renderMarkdown(message.content)}
          </div>
        </div>
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
