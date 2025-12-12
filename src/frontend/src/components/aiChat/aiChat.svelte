<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

  interface Message {
    role: 'user' | 'assistant'
    content: string
    timestamp: Date
  }

  interface ChatCompletionRequest {
    messages: Array<{
      role: 'user' | 'assistant' | 'system'
      content: string
    }>
    stream?: boolean
    temperature?: number
    max_tokens?: number
    [key: string]: unknown
  }

  interface ChatCompletionChunk {
    choices: Array<{
      delta?: {
        content?: string
      }
      message?: {
        content?: string
      }
    }>
    model?: string
  }

  let messages: Message[] = []
  let inputMessage = ''
  let loading = false
  let error = ''
  let abortController: AbortController | null = null
  let currentStreamingContent = ''

  // Configuration
  const API_ENDPOINT = '/v1/chat/completions' // llama.cpp API endpoint
  const STREAM = true
  const TEMPERATURE = 0.8
  const MAX_TOKENS = -1 // -1 means infinite

  const sendMessage = async () => {
    if (!inputMessage.trim() || loading) return

    const userMessage: Message = {
      role: 'user',
      content: inputMessage.trim(),
      timestamp: new Date()
    }

    messages = [...messages, userMessage]
    const currentInput = inputMessage.trim()
    inputMessage = ''
    loading = true
    error = ''
    currentStreamingContent = ''

    // Create abort controller for this request
    abortController = new AbortController()

    try {
      // Build messages array for API
      const apiMessages = messages.map((msg) => ({
        role: msg.role,
        content: msg.content
      }))

      const requestBody: ChatCompletionRequest = {
        messages: apiMessages,
        stream: STREAM,
        temperature: TEMPERATURE,
        max_tokens: MAX_TOKENS
      }

      if (STREAM) {
        await streamChatCompletion(requestBody)
      } else {
        await nonStreamChatCompletion(requestBody)
      }
    } catch (err: any) {
      if (err.name === 'AbortError') {
        // Request was aborted, save partial response if any
        if (currentStreamingContent.trim()) {
          const assistantMessage: Message = {
            role: 'assistant',
            content: currentStreamingContent,
            timestamp: new Date()
          }
          messages = [...messages, assistantMessage]
        }
        return
      }

      error = err.response?.data?.error?.message || err.message || 'Failed to send message'
      console.error('Chat error:', err)
    } finally {
      loading = false
      abortController = null
      currentStreamingContent = ''
    }
  }

  const streamChatCompletion = async (requestBody: ChatCompletionRequest) => {
    // Use axios with responseType: 'stream' for streaming
    // Note: axios doesn't support streaming well, so we'll use fetch with axios baseURL
    const baseURL = axiosBackendInstance.defaults.baseURL || ''
    const url = `${baseURL}${API_ENDPOINT}`

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...axiosBackendInstance.defaults.headers.common
      },
      body: JSON.stringify(requestBody),
      signal: abortController?.signal
    })

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}))
      throw new Error(errorData.error?.message || `HTTP ${response.status}: ${response.statusText}`)
    }

    const reader = response.body?.getReader()
    if (!reader) {
      throw new Error('No response body')
    }

    const decoder = new TextDecoder()
    let buffer = ''

    try {
      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        buffer += decoder.decode(value, { stream: true })
        const lines = buffer.split('\n')
        buffer = lines.pop() || ''

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6)
            if (data === '[DONE]') {
              // Stream complete
              if (currentStreamingContent.trim()) {
                const assistantMessage: Message = {
                  role: 'assistant',
                  content: currentStreamingContent,
                  timestamp: new Date()
                }
                messages = [...messages, assistantMessage]
              }
              return
            }

            try {
              const parsed: ChatCompletionChunk = JSON.parse(data)
              const content = parsed.choices[0]?.delta?.content

              if (content) {
                currentStreamingContent += content
                // Update the last message in real-time (create if doesn't exist)
                if (messages.length > 0 && messages[messages.length - 1].role === 'user') {
                  // Add assistant message if it doesn't exist yet
                  const assistantMessage: Message = {
                    role: 'assistant',
                    content: currentStreamingContent,
                    timestamp: new Date()
                  }
                  messages = [...messages, assistantMessage]
                } else if (messages.length > 0 && messages[messages.length - 1].role === 'assistant') {
                  // Update existing assistant message
                  messages = messages.map((msg, idx) =>
                    idx === messages.length - 1
                      ? { ...msg, content: currentStreamingContent }
                      : msg
                  )
                }
              }
            } catch (e) {
              console.error('Error parsing chunk:', e)
            }
          }
        }
      }
    } finally {
      reader.releaseLock()
    }
  }

  const nonStreamChatCompletion = async (requestBody: ChatCompletionRequest) => {
    const response = await axiosBackendInstance.post(API_ENDPOINT, requestBody, {
      signal: abortController?.signal
    })

    const content = response.data.choices?.[0]?.message?.content || ''
    if (content) {
      const assistantMessage: Message = {
        role: 'assistant',
        content,
        timestamp: new Date()
      }
      messages = [...messages, assistantMessage]
    }
  }

  const stopGeneration = () => {
    if (abortController) {
      abortController.abort()
      abortController = null
    }
  }

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      if (loading) {
        stopGeneration()
      } else {
        sendMessage()
      }
    }
  }

  const clearChat = () => {
    messages = []
    error = ''
    currentStreamingContent = ''
    if (abortController) {
      abortController.abort()
      abortController = null
    }
  }
</script>

<div class="ai-chat">
  <div class="chat-header">
    <h3>AI Chat Assistant</h3>
    <div class="header-actions">
      {#if loading}
        <button onclick={stopGeneration} class="stop-button" title="Stop generation">
          Stop
        </button>
      {/if}
      {#if messages.length > 0}
        <button onclick={clearChat} class="clear-button" title="Clear chat">
          Clear
        </button>
      {/if}
    </div>
  </div>

  <div class="messages-container">
    {#if messages.length === 0}
      <div class="empty-state">
        <p>👋 Start a conversation with the AI assistant</p>
        <p class="hint">Ask questions, get help, or request information</p>
        <p class="hint-small">Press Enter to send, Shift+Enter for new line</p>
      </div>
    {:else}
      {#each messages as message (message.timestamp)}
        <div class="message {message.role}">
          <div class="message-header">
            <span class="message-role">{message.role === 'user' ? 'You' : 'Assistant'}</span>
            <span class="message-time">
              {message.timestamp.toLocaleTimeString([], {
                hour: '2-digit',
                minute: '2-digit'
              })}
            </span>
          </div>
          <div class="message-content">{message.content}</div>
        </div>
      {/each}
      {#if loading && currentStreamingContent}
        <div class="message assistant loading">
          <div class="message-header">
            <span class="message-role">Assistant</span>
          </div>
          <div class="message-content">
            {currentStreamingContent}
            <span class="typing-cursor">▊</span>
          </div>
        </div>
      {/if}
    {/if}
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="input-container">
    <textarea
      bind:value={inputMessage}
      onkeypress={handleKeyPress}
      placeholder="Type your message here... (Press Enter to send, Shift+Enter for new line)"
      disabled={loading}
      class="chat-input"
      rows="3"
    ></textarea>
    <button
      onclick={loading ? stopGeneration : sendMessage}
      disabled={!inputMessage.trim() && !loading}
      class="send-button"
    >
      {loading ? 'Stop' : 'Send'}
    </button>
  </div>
</div>

<style>
  .ai-chat {
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    height: 600px;
    background-color: #fff;
    border: 1px solid #ddd;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 1rem;
    border-bottom: 2px solid #f0f0f0;
    margin-bottom: 1rem;
  }

  .chat-header h3 {
    margin: 0;
    color: #100f0f;
    font-size: 1.5rem;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .clear-button,
  .stop-button {
    padding: 0.5rem 1rem;
    background-color: #f5f5f5;
    color: #666;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .clear-button:hover,
  .stop-button:hover {
    background-color: #e8e8e8;
    color: #333;
  }

  .stop-button {
    background-color: #fee;
    color: #c33;
    border-color: #fcc;
  }

  .stop-button:hover {
    background-color: #fdd;
  }

  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
    text-align: center;
  }

  .empty-state p {
    margin: 0.5rem 0;
  }

  .empty-state .hint {
    font-size: 0.9rem;
    color: #999;
  }

  .empty-state .hint-small {
    font-size: 0.8rem;
    color: #aaa;
  }

  .message {
    display: flex;
    flex-direction: column;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    max-width: 80%;
    animation: fadeIn 0.3s ease-in;
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

  .message.user {
    align-self: flex-end;
    background-color: #b12424;
    color: white;
  }

  .message.assistant {
    align-self: flex-start;
    background-color: #f5f5f5;
    color: #333;
  }

  .message.loading {
    background-color: #f5f5f5;
  }

  .message-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
    font-size: 0.85rem;
    opacity: 0.8;
  }

  .message-role {
    font-weight: 600;
  }

  .message-time {
    font-size: 0.75rem;
    opacity: 0.7;
  }

  .message-content {
    line-height: 1.5;
    word-wrap: break-word;
    white-space: pre-wrap;
  }

  .typing-cursor {
    display: inline-block;
    animation: blink 1s infinite;
    margin-left: 2px;
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

  .error {
    padding: 0.75rem;
    background-color: #fee;
    border: 1px solid #fcc;
    border-radius: 4px;
    color: #c33;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }

  .input-container {
    display: flex;
    gap: 0.5rem;
    padding-top: 1rem;
    border-top: 2px solid #f0f0f0;
  }

  .chat-input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 1rem;
    font-family: inherit;
    resize: vertical;
    min-height: 60px;
  }

  .chat-input:focus {
    outline: none;
    border-color: #b12424;
  }

  .chat-input:disabled {
    background-color: #f5f5f5;
    cursor: not-allowed;
  }

  .send-button {
    padding: 0.75rem 1.5rem;
    background-color: #b12424;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 1rem;
    cursor: pointer;
    transition: background-color 0.2s;
    align-self: flex-end;
    white-space: nowrap;
  }

  .send-button:hover:not(:disabled) {
    background-color: #8a1c1c;
  }

  .send-button:disabled {
    background-color: #ccc;
    cursor: not-allowed;
  }

  @media screen and (max-width: 768px) {
    .ai-chat {
      height: 500px;
    }

    .message {
      max-width: 90%;
    }
  }
</style>
