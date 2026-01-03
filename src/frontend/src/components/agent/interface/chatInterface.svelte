<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { useAgentWebSocket } from '@hooks/useAgentWebSocket'
  import { useTextToSpeech } from '@hooks/useTextToSpeech.svelte'
  import { activeTools as activeToolsStore } from '@stores/activeTools'
  import { chatLayout } from '@stores/chatLayout'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import type {
    ChatMessage,
    AgentStreamEvent,
    ModelCapabilities,
    FileAttachment
  } from '@types'
  import { generateMessageId, cleanTextForSpeech } from '../utils/formatting'
  import ChatHeader from './chat/ChatHeader.svelte'
  import ChatMessages from './chat/ChatMessages.svelte'
  import ChatInput from './chat/ChatInput.svelte'

  let {
    currentConversationId = undefined,
    loading = $bindable(false)
  }: {
    currentConversationId?: string
    loading?: boolean
  } = $props()

  const dispatch = createEventDispatcher<{
    newChat: void
    conversationCreated: string
    responseComplete: { usage: any; content: string }
  }>()

  let messages: ChatMessage[] = $state([])
  let inputMessage: string = $state('')
  let quotedMessage: string = $state('')
  let error: string = $state('')
  let conversationId: string | null = $state(null)
  let chatContainer: HTMLDivElement = $state()
  let currentStreamingMessage: string = $state('')
  let streamingMessageId: string | null = $state(null)

  const tts = useTextToSpeech()
  let ttsEnabled = $state(false)

  // Subscribe to active tools store - use $derived for reactivity
  let activeToolsSet: Set<string> = $state(new Set())
  let activeToolsList = $derived(Array.from(activeToolsSet))

  // Abort controller for stopping generation
  let abortController: AbortController | null = null
  let ignoringStream = false

  // Model capabilities
  let modelCapabilities: ModelCapabilities = $state({
    vision: false,
    audio: false
  })

  // Model name and token info
  let modelName: string = $state('')
  let ctxSize: number = $state(0)
  let tokenUsage: {
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  } | null = $state(null)

  // Track attachments from ChatInput
  let currentAttachments: FileAttachment[] = $state([])

  onMount(() => {
    const unsubscribe = activeToolsStore.subscribe((tools) => {
      activeToolsSet = tools
    })
    return () => unsubscribe()
  })

  // WebSocket for real-time agent updates
  const agentWs = useAgentWebSocket(
    (event) => {
      handleStreamEvent(event)
    },
    (err) => {
      console.error('Agent WebSocket error:', err)
    }
  )

  onMount(() => {
    // Connect to agent WebSocket for real-time updates
    agentWs.connect()
    // Fetch model capabilities
    fetchModelCapabilities()
    // Fetch model name and context size
    fetchModelInfo()
  })

  const fetchModelCapabilities = async () => {
    try {
      const response = await axiosBackendInstance.get<ModelCapabilities>(
        'agent/model-capabilities'
      )
      modelCapabilities = response.data
      // eslint-disable-next-line no-console
      console.log('📊 Model capabilities:', $state.snapshot(modelCapabilities))
    } catch (err: any) {
      console.error('⚠️ Failed to fetch model capabilities:', err)
      // Default to no capabilities if fetch fails
      modelCapabilities = { vision: false, audio: false }
    }
  }

  const fetchModelInfo = async () => {
    try {
      const response = await axiosBackendInstance.get<{
        hf_model: string
        ctx_size: number
      }>('llama-server/config')
      modelName = response.data.hf_model || 'Unknown'
      ctxSize = response.data.ctx_size || 0
      // eslint-disable-next-line no-console
      console.log('🤖 Model info:', { modelName, ctxSize })
    } catch (err: any) {
      console.error('⚠️ Failed to fetch model info:', err)
      modelName = 'Unknown'
      ctxSize = 0
    }
  }

  onDestroy(() => {
    // Disconnect WebSocket when component is destroyed
    agentWs.disconnect()

    if (typeof window !== 'undefined') {
      window.removeEventListener('mousemove', handleMouseMove)
      window.removeEventListener('mouseup', handleMouseUp)
    }
  })

  // Resize logic
  let isResizing = $state(false)
  let resizeMode: 'vertical' | 'horizontal' | 'both' = 'vertical'
  let startX = 0
  let startY = 0
  let startWidth = 0
  let startHeight = 0

  const handleResizeStart = (
    e: MouseEvent,
    mode: 'vertical' | 'horizontal' | 'both'
  ) => {
    isResizing = true
    resizeMode = mode
    startX = e.clientX
    startY = e.clientY
    startWidth = $chatLayout.width || 1000
    startHeight = $chatLayout.height

    window.addEventListener('mousemove', handleMouseMove)
    window.addEventListener('mouseup', handleMouseUp)
    e.preventDefault() // Prevent text selection
  }

  const handleMouseMove = (e: MouseEvent) => {
    if (!isResizing) return

    if (resizeMode === 'vertical' || resizeMode === 'both') {
      const dy = e.clientY - startY
      const newHeight = startHeight + dy
      // Lower min height (150px), no max limits to allow page scrolling
      if (newHeight >= 150) {
        chatLayout.setHeight(newHeight)
      }
    }

    if (resizeMode === 'horizontal' || resizeMode === 'both') {
      const dx = e.clientX - startX
      const newWidth = startWidth + dx
      // Min width 320px, Max width constrained by window
      if (newWidth >= 320 && newWidth < window.innerWidth - 20) {
        chatLayout.setWidth(newWidth)
      }
    }
  }

  const handleMouseUp = () => {
    isResizing = false
    window.removeEventListener('mousemove', handleMouseMove)
    window.removeEventListener('mouseup', handleMouseUp)
  }

  export const setInputMessage = (message: string) => {
    inputMessage = message
  }

  export const sendMessage = async (overrideMessage?: string) => {
    const msgToSend = overrideMessage || inputMessage
    if ((!msgToSend.trim() && currentAttachments.length === 0) || loading)
      return
    if (overrideMessage) inputMessage = overrideMessage

    let requestPayload: string | any[] = inputMessage.trim()
    const hasImages = currentAttachments.some((a) => a.type === 'image')

    if (hasImages) {
      const parts: any[] = []

      // Add user input text first
      if (inputMessage.trim()) {
        parts.push({ type: 'text', text: inputMessage.trim() })
      }

      for (const att of currentAttachments) {
        if (att.type === 'image' && att.content) {
          parts.push({
            type: 'image_url',
            image_url: { url: att.content }
          })
        } else if (att.content) {
          // For non-image attachments (text, pdf, etc), separate them clearly
          // Use a text part for each
          let header = `\n\n[File: ${att.name}]`
          if (att.type === 'pdf') header = `\n\n[PDF: ${att.name}]`
          else if (att.type === 'audio') header = `\n\n[Audio: ${att.name}]`

          parts.push({
            type: 'text',
            text: `${header}\n${att.content}\n\n`
          })
        }
      }
      requestPayload = parts
    } else if (currentAttachments.length > 0) {
      // Logic for text-only attachments (keep as string to be safe/simple or use array)
      // Let's stick to the existing string appending for text-only to minimize risk,
      // though backend now handles both.
      let textContent = inputMessage.trim()
      const attachmentTexts: string[] = []
      for (const att of currentAttachments) {
        if (att.content) {
          if (att.type === 'text') {
            attachmentTexts.push(`\n\n[File: ${att.name}]\n${att.content}\n\n`)
          } else if (att.type === 'pdf') {
            attachmentTexts.push(`\n\n[PDF: ${att.name}]\n${att.content}\n\n`)
          } else if (att.type === 'audio') {
            attachmentTexts.push(`\n\n[Audio File: ${att.name}]\n\n`)
          }
        }
      }
      requestPayload = textContent + attachmentTexts.join('')
    }

    const userMessage: ChatMessage = {
      id: generateMessageId(),
      role: 'user',
      content: requestPayload,
      timestamp: Date.now(),
      attachments:
        currentAttachments.length > 0 ? [...currentAttachments] : undefined
    }

    messages = [...messages, userMessage]
    const currentInput = requestPayload // string or object
    inputMessage = ''
    // Clear attachments after building message (ChatInput will also clear them)
    currentAttachments = []
    loading = true
    error = ''
    currentStreamingMessage = ''
    streamingMessageId = null

    // Scroll to bottom
    setTimeout(() => scrollToBottom(true), 100)

    try {
      // Create new abort controller
      if (abortController) abortController.abort()
      abortController = new AbortController()
      ignoringStream = false

      // Cast to any because specific TS interface might expect string
      const request: any = {
        message: currentInput,
        conversation_id: conversationId || undefined
      }

      // Send message

      const baseUrl = axiosBackendInstance.defaults.baseURL || ''
      const response = await window.fetch(`${baseUrl}/agent/chat/stream`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(request),
        signal: abortController.signal
      })

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`)
      }
      // Scroll to bottom
      setTimeout(() => scrollToBottom(true), 100)
    } catch (err: any) {
      if (err.name === 'AbortError') {
        return
      }
      console.error('Failed to send message:', err)
      loading = false
      error =
        err.response?.data?.error ||
        err.response?.data?.message ||
        err.message ||
        'Failed to send message'
    }
  }

  const stopGeneration = async () => {
    // Do not abort the controller or ignore the stream.
    // We want to receive the final events (like Done with conversation_id) from the backend
    // after it processes the cancellation.

    // Explicitly signal backend to stop, if we have a conversation ID
    if (conversationId) {
      try {
        await axiosBackendInstance.post(`agent/chat/${conversationId}/cancel`)

        // Optimistically update status to show we are stopping
        const statusIndex = messages.findIndex((m) => m.role === 'status')
        if (statusIndex >= 0) {
          messages[statusIndex] = {
            ...messages[statusIndex],
            statusType: 'tool_error',
            content: 'Stopping generation...'
          }
        }
      } catch (err) {
        console.error('Failed to send explicit cancel signal to backend:', err)
        // Fallback: If backend is unreachable, then we MUST abort locally
        if (abortController) {
          abortController.abort()
          abortController = null
        }
        loading = false
      }
    } else {
      // If we don't have a conversation ID yet (rare race condition at very start),
      // we have to abort locally because we can't tell backend what to cancel.
      if (abortController) {
        abortController.abort()
        abortController = null
      }
      loading = false
    }
  }

  // Auto-scroll function
  const scrollToBottom = (smooth = false) => {
    if (chatContainer) {
      if (smooth) {
        chatContainer.scrollTo({
          top: chatContainer.scrollHeight,
          behavior: 'smooth'
        })
      } else {
        chatContainer.scrollTop = chatContainer.scrollHeight
      }
    }
  }

  const handleStreamEvent = (event: AgentStreamEvent) => {
    if (ignoringStream) return

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
          setTimeout(() => scrollToBottom(), 10)
        }
        break

      case 'tool_call':
        // Tool execution is shown in messages, no need to track in store for badges
        // Badges only show enabled tools from config
        // Remove any existing tool message for this tool
        messages = messages.filter(
          (m) => m.role !== 'tool' || m.toolName !== event.tool_name
        )
        messages.push({
          id: generateMessageId(),
          role: 'tool',
          content: `Calling ${event.display_name || event.tool_name}...`,
          timestamp: Date.now(),
          toolName: event.tool_name
        })
        break

      case 'tool_result': {
        // Tool execution is shown in messages, no need to track in store for badges
        // Remove status message when tool completes
        messages = messages.filter((m) => m.role !== 'status')
        // Update existing tool message or create new one
        const toolIndex = messages.findIndex(
          (m) => m.role === 'tool' && m.toolName === event.tool_name
        )
        const nameToUse = event.display_name || event.tool_name || 'Tool'
        if (toolIndex >= 0) {
          messages[toolIndex].content = event.success
            ? `✅ ${nameToUse} completed`
            : `${nameToUse} failed: ${event.result || 'Unknown error'}`
        } else {
          messages.push({
            id: generateMessageId(),
            role: 'tool',
            content: event.success
              ? `✅ ${nameToUse} completed`
              : `${nameToUse} failed: ${event.result || 'Unknown error'}`,
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
          setTimeout(() => scrollToBottom(), 10)
        }
        break

      case 'done':
        loading = false
        if (event.conversation_id) {
          const isNewConversation = conversationId === null
          conversationId = event.conversation_id

          if (isNewConversation) {
            dispatch('conversationCreated', conversationId)
          }
        }
        if (event.usage) {
          tokenUsage = event.usage
        }

        dispatch('responseComplete', {
          usage: tokenUsage,
          content: currentStreamingMessage
        })

        // Remove any remaining status messages
        messages = messages.filter((m) => m.role !== 'status')
        // Mark streaming message as complete
        if (streamingMessageId) {
          const streamingIndex = messages.findIndex(
            (m) => m.id === streamingMessageId
          )
          if (streamingIndex >= 0) {
            messages[streamingIndex].timestamp = Date.now()

            // Capture final content from message to be sure
            const finalContent = messages[streamingIndex].content as string
            dispatch('responseComplete', {
              usage: tokenUsage,
              content: finalContent
            })

            // Speak the message if TTS is enabled
            if (ttsEnabled && tts.isSupported && currentStreamingMessage) {
              const speechText = cleanTextForSpeech(currentStreamingMessage)
              if (speechText) {
                tts.speak(speechText)
              }
            }
          }
          streamingMessageId = null
        }
        currentStreamingMessage = ''
        break

      case 'error':
        loading = false
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

  const clearChat = () => {
    messages = []
    conversationId = null
    error = ''
    tokenUsage = null
    dispatch('newChat')
  }

  const handleInputChange = (value: string) => {
    inputMessage = value
  }

  const handleQuote = (text: string) => {
    quotedMessage = text
  }

  const handleClearQuote = () => {
    quotedMessage = ''
  }

  const loadMessages = async (id: string) => {
    loading = true
    try {
      const response = await axiosBackendInstance.get<ChatMessage[]>(
        `agent/conversations/${id}/messages`
      )
      messages = response.data.map((m: any) => ({
        id: generateMessageId(),
        role: m.role as any,
        content: m.content || '',
        timestamp: Date.now(),
        toolName: m.name
      }))
      conversationId = id
    } catch (err) {
      console.error('Failed to load messages:', err)
      error = 'Failed to load conversation history'
    } finally {
      loading = false
      setTimeout(() => scrollToBottom(), 100)
    }
  }

  // Track previous ID to detect changes
  let prevConversationId: string | undefined = $state(undefined)

  $effect(() => {
    if (currentConversationId !== prevConversationId) {
      prevConversationId = currentConversationId

      if (currentConversationId) {
        loadMessages(currentConversationId)
      } else {
        messages = []
        conversationId = null
        error = ''
        tokenUsage = null
      }
    }
  })

  // Auto-scroll when messages change
  $effect(() => {
    if (messages.length > 0) {
      setTimeout(() => scrollToBottom(), 50)
    }
  })
</script>

<div
  class="chat-interface"
  style:height="{$chatLayout.height}px"
  style:width={$chatLayout.width ? `${$chatLayout.width}px` : '100%'}
  style:transition={isResizing ? 'none' : 'height 0.2s ease, width 0.2s ease'}
>
  <ChatHeader
    {activeToolsList}
    hasMessages={messages.length > 0}
    onClear={clearChat}
    {modelName}
  />

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <ChatMessages {messages} {loading} bind:chatContainer onQuote={handleQuote} />

  <ChatInput
    bind:inputMessage
    {loading}
    {modelCapabilities}
    onSend={sendMessage}
    onStop={stopGeneration}
    onInputChange={handleInputChange}
    onAttachmentsChange={(attachments) => {
      currentAttachments = attachments
    }}
    {quotedMessage}
    onClearQuote={handleClearQuote}
    {ttsEnabled}
    onToggleTTS={() => (ttsEnabled = !ttsEnabled)}
    ttsSpeaking={tts.isSpeaking}
    onStopTTS={tts.cancel}
    {tokenUsage}
    {ctxSize}
  />

  <div
    class="resize-handle bottom"
    onmousedown={(e) => handleResizeStart(e, 'vertical')}
    role="button"
    tabindex="0"
    aria-label="Resize chat height"
  >
    <MaterialIcon name="drag-horizontal" width="24" height="24" />
  </div>

  <div
    class="resize-handle right"
    onmousedown={(e) => handleResizeStart(e, 'horizontal')}
    role="button"
    tabindex="0"
    aria-label="Resize chat width"
  >
    <div class="handle-bar"></div>
  </div>

  <div
    class="resize-handle corner"
    onmousedown={(e) => handleResizeStart(e, 'both')}
    role="button"
    tabindex="0"
    aria-label="Resize chat both directions"
  >
    <MaterialIcon name="resize-bottom-right" width="16" height="16" />
  </div>
</div>

<style>
  .chat-interface {
    display: flex;
    flex-direction: column;
    /* height is handled by inline style */
    background-color: var(--bg-primary, #fff);
    padding: 0;
    padding: 0;
    /* width is handled by inline style */
    max-width: none; /* remove default constraint */
    margin: 0 auto;
    position: relative; /* for absolute positioning of handles */
    border-radius: 8px;
    box-shadow: 0 2px 8px var(--shadow, rgba(0, 0, 0, 0.1));
    border: 1px solid var(--border-color, #e0e0e0);
    overflow: hidden;
  }

  .error {
    padding: 0.75rem 1rem;
    background-color: rgba(255, 200, 200, 0.2);
    border-bottom: 1px solid rgba(255, 100, 100, 0.5);
    color: var(--accent-color, #c33);
    font-size: 0.9rem;
  }

  @media screen and (max-width: 768px) {
    .chat-interface {
      padding: 0.5rem;
    }
  }

  .resize-handle {
    background-color: transparent;
    transition: background-color 0.2s;
    user-select: none;
    z-index: 10;
  }

  .resize-handle.bottom {
    height: 16px;
    background-color: var(--bg-secondary, #f5f5f5);
    border-top: 1px solid var(--border-color, #e0e0e0);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: ns-resize;
    color: var(--text-tertiary, #999);
    flex-shrink: 0;
  }

  .resize-handle.bottom:hover,
  .resize-handle.bottom:active {
    background-color: var(--border-color, #e0e0e0);
    color: var(--text-secondary, #666);
  }

  .resize-handle.right {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 16px; /* Space for corner handle */
    width: 8px;
    cursor: ew-resize;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .resize-handle.right:hover,
  .resize-handle.right:active {
    background-color: rgba(0, 0, 0, 0.05);
  }

  .resize-handle.right .handle-bar {
    width: 2px;
    height: 20px;
    background-color: var(--border-color, #e0e0e0);
    border-radius: 2px;
  }

  .resize-handle.corner {
    position: absolute;
    bottom: 0;
    right: 0;
    width: 16px;
    height: 16px;
    cursor: nwse-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--bg-secondary, #f5f5f5);
    border-top-left-radius: 4px;
    color: var(--text-tertiary, #999);
  }

  .resize-handle.corner:hover,
  .resize-handle.corner:active {
    background-color: var(--border-color, #e0e0e0);
    color: var(--text-secondary, #666);
  }
</style>
