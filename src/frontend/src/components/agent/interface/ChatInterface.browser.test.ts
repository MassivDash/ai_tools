/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import ChatInterface from './ChatInterface.svelte'
import { axiosBackendInstance } from '../../../axiosInstance/axiosBackendInstance.ts'
import type { Component } from 'svelte'
import { clearToolsCache } from '../utils/toolIcons'

// Mock axiosBackendInstance
vi.mock('../../../axiosInstance/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn(),
    defaults: { baseURL: 'http://localhost:8000' }
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
}

// Mock WebSocket hook
const mocks = vi.hoisted(() => {
  const wsConnect = vi.fn()
  const wsDisconnect = vi.fn()
  const wsSend = vi.fn()
  const mockAgentWs = {
    connect: wsConnect,
    disconnect: wsDisconnect,
    send: wsSend
  }
  return {
    wsConnect,
    wsDisconnect,
    wsSend,
    mockAgentWs
  }
})

// We need to capture the event handler passed to useAgentWebSocket
let wsEventHandler: (_event: any) => void = () => {}
let wsErrorHandler: (_err: any) => void = () => {}

vi.mock('@hooks/useAgentWebSocket', () => ({
  useAgentWebSocket: vi.fn((handler, onError) => {
    wsEventHandler = handler
    wsErrorHandler = onError
    return mocks.mockAgentWs
  })
}))

// Mock activeTools store
vi.mock('@stores/activeTools', () => ({
  activeTools: {
    subscribe: vi.fn((run) => {
      run(new Set(['calculator']))
      return () => {}
    })
  }
}))

// Mock window.fetch for streaming response
globalThis.fetch = vi.fn()

// jsdom does not implement Element.prototype.scrollTo, and ChatInterface's
// auto-scroll timers call it unconditionally as soon as a message exists.
;(window.Element.prototype as any).scrollTo = function () {}

let errorSpy: ReturnType<typeof vi.spyOn>

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

const wait = (ms = 0) => new Promise((r) => setTimeout(r, ms))

const textarea = () =>
  screen.getByPlaceholderText(/Type your message/) as HTMLTextAreaElement

const sendButton = () =>
  document.querySelector('.send-button:not(.stop-button)') as HTMLElement

const stopButton = () => document.querySelector('.stop-button') as HTMLElement

const errorText = () => document.querySelector('.error')?.textContent?.trim()

const fileInputs = () =>
  Array.from(
    document.querySelectorAll('input[type="file"]')
  ) as HTMLInputElement[]

/**
 * jsdom's Blob/File has no `.text()`, which ChatInput uses to read text
 * attachments, so provide it on the instances the tests hand over.
 */
const makeFile = (content: string, name: string, type: string) => {
  const file = new File([content], name, { type })
  if (typeof (file as any).text !== 'function') {
    ;(file as any).text = () => Promise.resolve(content)
  }
  return file
}

/**
 * Feed a File into one of ChatInput's hidden <input type="file"> elements.
 *
 * ChatInput's handleFileSelect is async (file.text(), the pdf-to-markdown POST,
 * FileReader/Image callbacks), and only calls onAttachmentsChange once the
 * content has been read. Draining the queue here means the caller's send is
 * guaranteed to see the fully-populated attachment rather than racing it, which
 * otherwise flakes when the machine is loaded.
 */
const attachFile = async (input: HTMLInputElement, file: File) => {
  Object.defineProperty(input, 'files', { value: [file], configurable: true })
  await fireEvent.change(input)
  // Two macrotask turns: enough for chained promises plus the setTimeout(0)
  // hops used by the FileReader/Image stubs.
  await new Promise((resolve) => setTimeout(resolve, 0))
  await new Promise((resolve) => setTimeout(resolve, 0))
}

/** Body of the Nth window.fetch call, parsed back from JSON. */
const sentBody = (call = 0) =>
  JSON.parse((globalThis.fetch as any).mock.calls[call][1].body)

const typeAndSend = async (text: string) => {
  await fireEvent.input(textarea(), { target: { value: text } })
  await fireEvent.click(sendButton())
}

/** jsdom has no canvas/image decoding; stub what ChatInput's processImage needs. */
const installImagePipelineStubs = () => {
  const originalImage = (window as any).Image
  const originalGetContext = HTMLCanvasElement.prototype.getContext
  const originalToDataURL = HTMLCanvasElement.prototype.toDataURL

  class StubImage {
    width = 800
    height = 600
    onload: (() => void) | null = null
    onerror: (() => void) | null = null
    _src = ''
    set src(value: string) {
      this._src = value
      setTimeout(() => this.onload?.(), 0)
    }
    get src() {
      return this._src
    }
  }
  ;(window as any).Image = StubImage
  HTMLCanvasElement.prototype.getContext = (() => ({
    fillStyle: '',
    fillRect: () => {},
    drawImage: () => {}
  })) as any
  HTMLCanvasElement.prototype.toDataURL = (() =>
    'data:image/jpeg;base64,STUBIMAGE') as any

  return () => {
    ;(window as any).Image = originalImage
    HTMLCanvasElement.prototype.getContext = originalGetContext
    HTMLCanvasElement.prototype.toDataURL = originalToDataURL
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  clearToolsCache() // Reset tool cache
  vi.spyOn(console, 'log').mockImplementation(() => {})
  errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

  // Default mocks
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/model-capabilities')
      return Promise.resolve({ data: { vision: true, audio: true } })
    if (url === 'llama-server/config')
      return Promise.resolve({
        data: { hf_model: 'test-model', ctx_size: 4096 }
      })
    if (url.includes('/messages')) return Promise.resolve({ data: [] }) // Empty history
    if (url.includes('agent/tools')) return Promise.resolve({ data: [] })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: {} })
  ;(globalThis.fetch as any).mockResolvedValue({
    ok: true,
    json: () => Promise.resolve({})
  })
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('loads initial state', async () => {
  render(ChatInterface as Component)

  await waitFor(() => {
    // Check header
    expect(screen.getByText(/^Chat$/)).toBeTruthy()
    // Check input present
    expect(screen.getByPlaceholderText(/Type your message/)).toBeTruthy()
  })

  expect(mocks.wsConnect).toHaveBeenCalled()
})

test('loads history when conversationId is provided', async () => {
  const historyMessages = [
    { role: 'user', content: 'History User' },
    { role: 'assistant', content: 'History Assistant', name: 'Agent' }
  ]

  mockedAxios.get.mockImplementation((url: string) => {
    if (url.includes('/messages'))
      return Promise.resolve({ data: historyMessages })
    if (url.includes('agent/tools')) return Promise.resolve({ data: [] })
    return Promise.resolve({ data: {} })
  })

  render(ChatInterface as Component, {
    props: { currentConversationId: '123' }
  })

  await waitFor(() => {
    expect(screen.getByText('History User')).toBeTruthy()
    expect(screen.getByText('History Assistant')).toBeTruthy()
  })
})

test('sends a message and handles optimistic update', async () => {
  ;(globalThis.fetch as any).mockResolvedValue({
    ok: true,
    json: () => Promise.resolve({})
  })

  render(ChatInterface as Component)

  const textarea = screen.getByPlaceholderText(/Type your message/)
  await fireEvent.input(textarea, { target: { value: 'Hello Agent' } })

  const btn = document.querySelector('.send-button')
  expect(btn).toBeTruthy()
  if (btn) await fireEvent.click(btn)

  expect(globalThis.fetch).toHaveBeenCalled()

  // Optimistic update should show message
  await waitFor(() => {
    expect(screen.getByText('Hello Agent')).toBeTruthy()
  })
})

test('displays incoming streaming text', async () => {
  render(ChatInterface as Component)

  // Simulate incoming text chunk via WS
  const event = {
    type: 'text_chunk',
    text: 'Streaming token'
  }

  // Wait for component to mount/connect
  await waitFor(() => expect(wsEventHandler).toBeDefined())

  // Trigger event
  wsEventHandler(event)

  await waitFor(() => {
    expect(screen.getByText('Streaming token')).toBeTruthy()
  })
})

test('smart scroll respects user position via scroll listener', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(wsEventHandler).toBeDefined())

  const scrollContainer = document.querySelector(
    '.chat-messages'
  ) as HTMLDivElement
  expect(scrollContainer).toBeTruthy()

  // Mock scrollTo
  scrollContainer.scrollTo = vi.fn()

  // Initial state: At bottom
  Object.defineProperty(scrollContainer, 'scrollTop', {
    value: 1000,
    writable: true
  })
  Object.defineProperty(scrollContainer, 'scrollHeight', {
    value: 1500,
    writable: true
  })
  Object.defineProperty(scrollContainer, 'clientHeight', {
    value: 500,
    writable: true
  })

  // 1. Simulate user scrolling up manually
  // distanceFromBottom = 1500 - 800 - 500 = 200px ( > 50px threshold)
  scrollContainer.scrollTop = 800
  await fireEvent.scroll(scrollContainer)

  // Incoming chunk should NOT trigger scrollTo
  wsEventHandler({ type: 'text_chunk', text: ' chunk' })

  await new Promise((r) => setTimeout(r, 50))
  expect(scrollContainer.scrollTo).not.toHaveBeenCalled()

  // 2. Simulate user scrolling back to bottom manually
  // distanceFromBottom = 0
  scrollContainer.scrollTop = 1000
  await fireEvent.scroll(scrollContainer)

  // Incoming chunk SHOULD trigger scrollTo
  wsEventHandler({ type: 'text_chunk', text: ' chunk 2' })

  await waitFor(() => {
    expect(scrollContainer.scrollTo).toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// bootstrap: capabilities / model info / websocket error callback
// ---------------------------------------------------------------------------

test('renders model name reported by llama-server/config', async () => {
  render(ChatInterface as Component)

  await waitFor(() => {
    expect(document.querySelector('.model-name')?.textContent).toBe(
      'test-model'
    )
  })
})

test('falls back to Unknown model when model info request fails', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config')
      return Promise.reject(new Error('config down'))
    if (url === 'agent/model-capabilities')
      return Promise.resolve({ data: { vision: true, audio: true } })
    return Promise.resolve({ data: {} })
  })

  render(ChatInterface as Component)

  await waitFor(() => {
    expect(document.querySelector('.model-name')?.textContent).toBe('Unknown')
  })
  expect(errorSpy).toHaveBeenCalledWith(
    '⚠️ Failed to fetch model info:',
    expect.any(Error)
  )
})

test('hides audio and image upload when capabilities request fails', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/model-capabilities')
      return Promise.reject(new Error('caps down'))
    if (url === 'llama-server/config')
      return Promise.resolve({ data: { hf_model: 'test-model', ctx_size: 10 } })
    return Promise.resolve({ data: {} })
  })

  render(ChatInterface as Component)

  await waitFor(() => {
    expect(errorSpy).toHaveBeenCalledWith(
      '⚠️ Failed to fetch model capabilities:',
      expect.any(Error)
    )
  })

  // vision/audio default to false -> those upload affordances are gone,
  // while the always-available text/pdf ones stay.
  expect(screen.queryByTitle('Upload image file')).toBeNull()
  expect(screen.queryByTitle('Upload audio file')).toBeNull()
  expect(screen.getByTitle('Upload text file (txt, md)')).toBeTruthy()
  expect(screen.getByTitle('Upload PDF file')).toBeTruthy()
  // and their hidden file inputs are not rendered either
  expect(fileInputs()).toHaveLength(2)
})

test('shows audio and image upload when the model supports them', async () => {
  render(ChatInterface as Component)

  await waitFor(() => {
    expect(screen.getByTitle('Upload image file')).toBeTruthy()
  })
  expect(screen.getByTitle('Upload audio file')).toBeTruthy()
  expect(fileInputs()).toHaveLength(4)
})

test('logs websocket errors reported by the agent socket hook', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsErrorHandler).toBe('function'))

  const failure = new Error('socket exploded')
  wsErrorHandler(failure)

  expect(errorSpy).toHaveBeenCalledWith('Agent WebSocket error:', failure)
})

test('disconnects the agent socket on destroy', async () => {
  const { unmount } = render(ChatInterface as Component)
  await waitFor(() => expect(mocks.wsConnect).toHaveBeenCalled())

  expect(mocks.wsDisconnect).not.toHaveBeenCalled()
  unmount()
  expect(mocks.wsDisconnect).toHaveBeenCalled()
})

// ---------------------------------------------------------------------------
// sendMessage: exported API, guards, error paths
// ---------------------------------------------------------------------------

test('setInputMessage fills the composer', async () => {
  const { component } = render(ChatInterface as Component)
  ;(component as any).setInputMessage('injected from parent')

  await waitFor(() => {
    expect(textarea().value).toBe('injected from parent')
  })
})

test('sendMessage(override) replaces the composer text and posts it', async () => {
  const { component } = render(ChatInterface as Component)

  await (component as any).sendMessage('override message')

  expect(globalThis.fetch).toHaveBeenCalledWith(
    'http://localhost:8000/agent/chat/stream',
    expect.objectContaining({ method: 'POST' })
  )
  expect(sentBody().message).toBe('override message')
  expect(sentBody().conversation_id).toBeUndefined()

  await waitFor(() => {
    expect(screen.getByText('override message')).toBeTruthy()
    expect(textarea().value).toBe('')
  })
})

test('sendMessage ignores empty input and re-entrant calls while loading', async () => {
  const { component } = render(ChatInterface as Component)

  // nothing typed, no attachments -> no request at all
  await (component as any).sendMessage()
  await (component as any).sendMessage('   ')
  expect(globalThis.fetch).not.toHaveBeenCalled()

  await (component as any).sendMessage('first')
  expect(globalThis.fetch).toHaveBeenCalledTimes(1)

  // loading is still true (no `done` event yet) -> second send is dropped
  await (component as any).sendMessage('second')
  expect(globalThis.fetch).toHaveBeenCalledTimes(1)
  expect(screen.queryByText('second')).toBeNull()
})

test('surfaces a non-ok stream response as an error and stops loading', async () => {
  ;(globalThis.fetch as any).mockResolvedValue({ ok: false, status: 500 })

  render(ChatInterface as Component)
  await typeAndSend('boom please')

  await waitFor(() => {
    expect(errorText()).toBe('HTTP error! status: 500')
  })
  // loading was reset, so the composer offers "send" again rather than "stop"
  expect(stopButton()).toBeNull()
  expect(sendButton()).toBeTruthy()
})

test('prefers the server-provided error message when sending fails', async () => {
  ;(globalThis.fetch as any).mockRejectedValue({
    response: { data: { error: 'agent is busy' } },
    message: 'Request failed'
  })

  render(ChatInterface as Component)
  await typeAndSend('hi')

  await waitFor(() => {
    expect(errorText()).toBe('agent is busy')
  })
})

test('a featureless send failure falls back to a generic message', async () => {
  ;(globalThis.fetch as any).mockRejectedValue({})

  render(ChatInterface as Component)
  await typeAndSend('hi')

  await waitFor(() => expect(errorText()).toBe('Failed to send message'))
})

test('an aborted stream request is not reported as an error', async () => {
  ;(globalThis.fetch as any).mockRejectedValue(
    Object.assign(new Error('The user aborted a request.'), {
      name: 'AbortError'
    })
  )

  render(ChatInterface as Component)
  await typeAndSend('cancel me')

  await waitFor(() => expect(screen.getByText('cancel me')).toBeTruthy())
  expect(errorText()).toBeUndefined()
  expect(errorSpy).not.toHaveBeenCalledWith(
    'Failed to send message:',
    expect.anything()
  )
  // loading was intentionally left on: the stop button is still offered
  expect(stopButton()).toBeTruthy()
})

// ---------------------------------------------------------------------------
// attachments -> request payload shaping
// ---------------------------------------------------------------------------

test('appends a text attachment to the outgoing string payload', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(fileInputs()).toHaveLength(4))

  await attachFile(
    fileInputs()[0],
    makeFile('line one\nline two', 'notes.txt', 'text/plain')
  )

  await waitFor(() => {
    expect(screen.getByText('notes.txt')).toBeTruthy()
  })

  await typeAndSend('summarise this')

  expect(sentBody().message).toBe(
    'summarise this\n\n[File: notes.txt]\nline one\nline two\n\n'
  )
})

test('converts a PDF attachment and labels it in the payload', async () => {
  mockedAxios.post.mockImplementation((url: string) => {
    if (url === 'pdf-to-markdown')
      return Promise.resolve({
        data: { markdown: '# Extracted', filename: 'doc.pdf' }
      })
    return Promise.resolve({ data: {} })
  })

  render(ChatInterface as Component)
  await waitFor(() => expect(fileInputs()).toHaveLength(4))

  await attachFile(
    fileInputs()[3],
    makeFile('%PDF-1.4', 'doc.pdf', 'application/pdf')
  )

  await waitFor(() => expect(screen.getByText('doc.pdf')).toBeTruthy())

  await typeAndSend('what is in here')

  expect(sentBody().message).toBe(
    'what is in here\n\n[PDF: doc.pdf]\n# Extracted\n\n'
  )
})

test('references an audio attachment by name only', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(fileInputs()).toHaveLength(4))

  await attachFile(
    fileInputs()[1],
    makeFile('fake-audio-bytes', 'clip.mp3', 'audio/mpeg')
  )

  await waitFor(() => expect(screen.getByText('clip.mp3')).toBeTruthy())

  await typeAndSend('transcribe')

  expect(sentBody().message).toBe('transcribe\n\n[Audio File: clip.mp3]\n\n')
})

test('builds a multipart payload when an image is attached', async () => {
  const restore = installImagePipelineStubs()
  try {
    render(ChatInterface as Component)
    await waitFor(() => expect(fileInputs()).toHaveLength(4))

    await attachFile(
      fileInputs()[2],
      makeFile('png-bytes', 'pic.png', 'image/png')
    )

    // ChatInput normalises the name to .jpg after re-encoding
    await waitFor(() => expect(screen.getByText('pic.jpg')).toBeTruthy())

    await typeAndSend('describe this')

    expect(sentBody().message).toEqual([
      { type: 'text', text: 'describe this' },
      {
        type: 'image_url',
        image_url: { url: 'data:image/jpeg;base64,STUBIMAGE' }
      }
    ])

    // the optimistic user bubble renders the inline image
    await waitFor(() => {
      const img = document.querySelector('img.message-image')
      expect(img?.getAttribute('src')).toBe('data:image/jpeg;base64,STUBIMAGE')
    })
  } finally {
    restore()
  }
})

test('mixes non-image attachments into the multipart payload as labelled text', async () => {
  const restore = installImagePipelineStubs()
  mockedAxios.post.mockImplementation((url: string) => {
    if (url === 'pdf-to-markdown')
      return Promise.resolve({ data: { markdown: 'pdf body' } })
    return Promise.resolve({ data: {} })
  })
  try {
    render(ChatInterface as Component)
    await waitFor(() => expect(fileInputs()).toHaveLength(4))

    await attachFile(
      fileInputs()[2],
      makeFile('png-bytes', 'pic.png', 'image/png')
    )
    await waitFor(() => expect(screen.getByText('pic.jpg')).toBeTruthy())

    await attachFile(
      fileInputs()[0],
      makeFile('text body', 'notes.txt', 'text/plain')
    )
    await waitFor(() => expect(screen.getByText('notes.txt')).toBeTruthy())

    await attachFile(
      fileInputs()[3],
      makeFile('%PDF-1.4', 'doc.pdf', 'application/pdf')
    )
    await waitFor(() => expect(screen.getByText('doc.pdf')).toBeTruthy())

    await attachFile(
      fileInputs()[1],
      makeFile('audio-bytes', 'clip.mp3', 'audio/mpeg')
    )
    await waitFor(() => expect(screen.getByText('clip.mp3')).toBeTruthy())

    // no typed text at all: the payload is attachments only
    await fireEvent.click(sendButton())

    expect(sentBody().message).toEqual([
      {
        type: 'image_url',
        image_url: { url: 'data:image/jpeg;base64,STUBIMAGE' }
      },
      { type: 'text', text: '\n\n[File: notes.txt]\ntext body\n\n' },
      { type: 'text', text: '\n\n[PDF: doc.pdf]\npdf body\n\n' },
      {
        type: 'text',
        text: expect.stringContaining('\n\n[Audio: clip.mp3]\n')
      }
    ])
  } finally {
    restore()
  }
})

// ---------------------------------------------------------------------------
// websocket stream events
// ---------------------------------------------------------------------------

test('shows general statuses and swaps them, but hides tool-lifecycle ones', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  // a status with no message is ignored entirely
  wsEventHandler({ type: 'status', status: 'thinking' })
  await wait(10)
  expect(document.querySelectorAll('.status-message')).toHaveLength(0)

  wsEventHandler({
    type: 'status',
    status: 'finalizing',
    message: 'Wrapping up'
  })
  await waitFor(() => expect(screen.getByText('Wrapping up')).toBeTruthy())

  // tool lifecycle statuses are deliberately skipped and must not replace the
  // general status that is already on screen
  wsEventHandler({
    type: 'status',
    status: 'tool_executing',
    message: 'Running calculator'
  })
  await wait(20)
  expect(screen.queryByText('Running calculator')).toBeNull()
  expect(screen.getByText('Wrapping up')).toBeTruthy()

  // a new general status replaces the previous one rather than stacking
  wsEventHandler({
    type: 'status',
    status: 'finalizing',
    message: 'Almost done'
  })
  await waitFor(() => expect(screen.getByText('Almost done')).toBeTruthy())
  expect(screen.queryByText('Wrapping up')).toBeNull()
  expect(document.querySelectorAll('.status-message')).toHaveLength(1)

  // a status with a message but no status type is still shown
  wsEventHandler({ type: 'status', message: 'Untyped status' })
  await waitFor(() => expect(screen.getByText('Untyped status')).toBeTruthy())
  expect(document.querySelectorAll('.status-message')).toHaveLength(1)
})

test('tool_call renders one bubble per tool and tool_result resolves it', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'status', status: 'finalizing', message: 'Thinking' })
  wsEventHandler({
    type: 'tool_call',
    tool_name: 'calculator',
    display_name: 'Calculator',
    tool_call_id: 'call-1',
    arguments: '{"a":1}'
  })
  await waitFor(() =>
    expect(screen.getByText('Calling Calculator...')).toBeTruthy()
  )

  // a repeated tool_call for the same tool replaces the existing bubble
  wsEventHandler({
    type: 'tool_call',
    tool_name: 'calculator',
    display_name: 'Calculator',
    tool_call_id: 'call-2'
  })
  await wait(20)
  expect(screen.getAllByText('Calling Calculator...')).toHaveLength(1)

  // tool_call with no display_name falls back to the raw tool name
  wsEventHandler({ type: 'tool_call', tool_name: 'web_search' })
  await waitFor(() =>
    expect(screen.getByText('Calling web_search...')).toBeTruthy()
  )

  wsEventHandler({
    type: 'tool_result',
    tool_name: 'calculator',
    display_name: 'Calculator',
    success: true
  })
  await waitFor(() =>
    expect(screen.getByText('Calculator completed')).toBeTruthy()
  )
  // the transient status bubble is dropped once a tool reports back
  expect(screen.queryByText('Thinking')).toBeNull()
  expect(screen.queryByText('Calling Calculator...')).toBeNull()

  // a failing result rewrites the same bubble in place
  wsEventHandler({
    type: 'tool_result',
    tool_name: 'web_search',
    success: false,
    result: 'rate limited'
  })
  await waitFor(() =>
    expect(screen.getByText('web_search failed: rate limited')).toBeTruthy()
  )
  expect(screen.queryByText('Calling web_search...')).toBeNull()
  expect(document.querySelectorAll('.tool-indicator')).toHaveLength(2)

  // ...and a failure with no result text gets a generic reason
  wsEventHandler({
    type: 'tool_result',
    tool_name: 'web_search',
    success: false
  })
  await waitFor(() =>
    expect(screen.getByText('web_search failed: Unknown error')).toBeTruthy()
  )
})

test('tool_result without a matching bubble creates a failure bubble', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({
    type: 'tool_result',
    tool_name: 'shell',
    success: false,
    result: 'permission denied'
  })
  await waitFor(() =>
    expect(screen.getByText('shell failed: permission denied')).toBeTruthy()
  )

  // no tool_name and no result at all -> generic labels
  wsEventHandler({ type: 'tool_result', success: false })
  await waitFor(() =>
    expect(screen.getByText('Tool failed: Unknown error')).toBeTruthy()
  )

  expect(document.querySelectorAll('.tool-indicator.error')).toHaveLength(2)

  // a success without a preceding tool_call also creates its own bubble
  wsEventHandler({ type: 'tool_result', tool_name: 'lookup', success: true })
  await waitFor(() => expect(screen.getByText('lookup completed')).toBeTruthy())
  expect(document.querySelectorAll('.tool-indicator.success')).toHaveLength(1)
})

test('text_chunk events accumulate into a single streaming bubble', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  // an empty chunk is a no-op
  wsEventHandler({ type: 'text_chunk', text: '' })
  await wait(10)
  expect(document.querySelectorAll('.message.assistant')).toHaveLength(0)

  wsEventHandler({ type: 'text_chunk', text: 'Hello' })
  wsEventHandler({ type: 'text_chunk', text: ' world' })

  await waitFor(() => expect(screen.getByText('Hello world')).toBeTruthy())
  expect(document.querySelectorAll('.message.streaming')).toHaveLength(1)
})

test('done completes the streaming bubble, reports usage and the conversation id', async () => {
  const created = vi.fn()
  const complete = vi.fn()
  // Svelte 5 delivers createEventDispatcher events through the $$events prop.
  render(ChatInterface as Component, {
    props: {
      $$events: { conversationCreated: created, responseComplete: complete }
    }
  })

  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'text_chunk', text: 'Final answer' })
  await waitFor(() => expect(screen.getByText('Final answer')).toBeTruthy())

  wsEventHandler({
    type: 'done',
    conversation_id: 'conv-1',
    usage: { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 }
  })

  await waitFor(() => {
    expect(document.querySelectorAll('.message.streaming')).toHaveLength(0)
  })
  expect(document.querySelectorAll('.message.assistant')).toHaveLength(1)

  expect(created).toHaveBeenCalledTimes(1)
  expect(created.mock.calls[0][0].detail).toBe('conv-1')
  expect(complete.mock.calls.at(-1)[0].detail).toEqual({
    usage: { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
    content: 'Final answer'
  })

  // token usage becomes visible in the composer footer
  await waitFor(() => {
    const usage = document
      .querySelector('.usage-text')
      ?.textContent?.replace(/\s+/g, ' ')
      .trim()
    expect(usage).toBe('15 / 4096 tokens (0%)')
  })

  // a second done for the same conversation is not a new conversation
  wsEventHandler({ type: 'done', conversation_id: 'conv-1' })
  await wait(10)
  expect(created).toHaveBeenCalledTimes(1)

  // subsequent sends carry the conversation id
  await typeAndSend('follow up')
  expect(sentBody().conversation_id).toBe('conv-1')
})

test('done with no payload just clears the loading state', async () => {
  render(ChatInterface as Component)
  await typeAndSend('anything')

  await waitFor(() => expect(stopButton()).toBeTruthy())
  wsEventHandler({ type: 'done' })
  await waitFor(() => expect(stopButton()).toBeNull())
  expect(sendButton()).toBeTruthy()
})

test('error events drop the partial answer and show the message', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'text_chunk', text: 'half an answ' })
  await waitFor(() => expect(screen.getByText('half an answ')).toBeTruthy())

  wsEventHandler({ type: 'error', message: 'model crashed' })

  await waitFor(() => expect(errorText()).toBe('model crashed'))
  expect(screen.queryByText('half an answ')).toBeNull()
  expect(document.querySelectorAll('.message.assistant')).toHaveLength(0)
})

test('error events without a message fall back to a generic error', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'error' })
  await waitFor(() => expect(errorText()).toBe('An error occurred'))
})

test('unknown stream event types are ignored', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'something_new', text: 'nope' })
  await wait(20)
  expect(
    screen.getByText(/Start a conversation with the AI agent/)
  ).toBeTruthy()
})

// ---------------------------------------------------------------------------
// clear chat / quoting
// ---------------------------------------------------------------------------

test('Clear Chat empties the transcript and resets the conversation', async () => {
  const newChat = vi.fn()
  render(ChatInterface as Component, {
    props: { $$events: { newChat } }
  })

  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))
  wsEventHandler({ type: 'text_chunk', text: 'Hello' })
  wsEventHandler({
    type: 'done',
    conversation_id: 'conv-7',
    usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 }
  })
  await waitFor(() => expect(screen.getByText('Hello')).toBeTruthy())

  await fireEvent.click(screen.getByText('Clear Chat'))

  await waitFor(() => {
    expect(
      screen.getByText(/Start a conversation with the AI agent/)
    ).toBeTruthy()
  })
  expect(newChat).toHaveBeenCalledTimes(1)
  expect(document.querySelector('.usage-text')).toBeNull()
  expect(screen.queryByText('Clear Chat')).toBeNull()

  // conversation id was reset, so the next send starts a fresh conversation
  await typeAndSend('brand new')
  expect(sentBody().conversation_id).toBeUndefined()
})

test('a chunk arriving after the transcript was cleared starts a new bubble', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'text_chunk', text: 'Partial' })
  await waitFor(() => expect(screen.getByText('Partial')).toBeTruthy())

  await fireEvent.click(screen.getByText('Clear Chat'))
  await waitFor(() => expect(screen.queryByText('Partial')).toBeNull())

  // the streaming id still points at a message that no longer exists, so the
  // next chunk has to recreate the bubble with the accumulated text
  wsEventHandler({ type: 'text_chunk', text: ' more' })
  await waitFor(() => expect(screen.getByText('Partial more')).toBeTruthy())

  // done cannot find the tracked message any more, but still stops loading
  wsEventHandler({ type: 'done' })
  await wait(10)
  expect(stopButton()).toBeNull()

  // and when the tracked bubble disappears before `done` arrives at all, the
  // event still settles without resurrecting it
  wsEventHandler({ type: 'text_chunk', text: 'Another' })
  await waitFor(() => expect(screen.getByText('Another')).toBeTruthy())
  await fireEvent.click(screen.getByText('Clear Chat'))
  wsEventHandler({ type: 'done', conversation_id: 'conv-late' })
  await wait(20)
  expect(
    screen.getByText(/Start a conversation with the AI agent/)
  ).toBeTruthy()
})

test('quoting a message fills the quote banner and it can be dismissed', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'text_chunk', text: 'Quote me please' })
  wsEventHandler({ type: 'done' })
  await waitFor(() => expect(screen.getByText('Quote me please')).toBeTruthy())

  await fireEvent.click(screen.getByTitle('Message options'))
  await fireEvent.click(screen.getByText('Quote'))

  await waitFor(() => {
    expect(document.querySelector('.quote-text')?.textContent).toBe(
      'Quote me please'
    )
  })

  await fireEvent.click(screen.getByLabelText('Dismiss quote'))
  await waitFor(() => {
    expect(document.querySelector('.quote-banner')).toBeNull()
  })
})

// ---------------------------------------------------------------------------
// ask_human / submitToolResult
// ---------------------------------------------------------------------------

test('answering an ask_human prompt posts the choice as a tool result', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({
    type: 'tool_call',
    tool_name: 'ask_human',
    tool_call_id: 'ask-1',
    arguments: JSON.stringify({ question: 'Pick one', options: ['Yes', 'No'] })
  })

  await waitFor(() => expect(screen.getByText('Pick one')).toBeTruthy())
  expect(screen.getByText('Waiting for your input...')).toBeTruthy()

  const radios = document.querySelectorAll(
    '.ask-human-option input[type="radio"]'
  )
  expect(radios).toHaveLength(3) // Yes / No / Other
  await fireEvent.click(radios[0])

  await fireEvent.click(screen.getByRole('button', { name: 'Submit' }))

  await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1))
  expect(sentBody()).toEqual({
    message: '',
    tool_result: {
      tool_name: 'ask_human',
      tool_call_id: 'ask-1',
      result: 'Yes'
    }
  })

  // the answer shows up optimistically as a user turn
  await waitFor(() => {
    expect(
      document
        .querySelector('.message.user .message-content')
        ?.textContent?.trim()
    ).toBe('Yes')
  })

  // a second submit while the first is still in flight is ignored
  await fireEvent.click(screen.getByRole('button', { name: 'Submit' }))
  expect(globalThis.fetch).toHaveBeenCalledTimes(1)
})

test('a non-ok tool-result response is surfaced as an error', async () => {
  ;(globalThis.fetch as any).mockResolvedValue({ ok: false, status: 502 })
  const { component } = render(ChatInterface as Component)

  await (component as any).submitToolResult('ask_human', 'ask-9', 'Maybe')

  await waitFor(() => expect(errorText()).toBe('HTTP error! status: 502'))
  expect(screen.getByText('Maybe')).toBeTruthy()
  expect(stopButton()).toBeNull()
})

test('a failed tool-result request prefers the server message', async () => {
  ;(globalThis.fetch as any).mockRejectedValue({
    response: { data: { message: 'tool result rejected' } }
  })
  const { component } = render(ChatInterface as Component)

  await (component as any).submitToolResult('ask_human', 'ask-9', 'Maybe')

  await waitFor(() => expect(errorText()).toBe('tool result rejected'))
  expect(errorSpy).toHaveBeenCalledWith(
    'Failed to submit tool result:',
    expect.anything()
  )
})

test('a featureless tool-result failure falls back to a generic message', async () => {
  ;(globalThis.fetch as any).mockRejectedValue({})
  const { component } = render(ChatInterface as Component)

  await (component as any).submitToolResult('ask_human', 'ask-9', 'Maybe')

  await waitFor(() => expect(errorText()).toBe('Failed to submit tool result'))
})

test('a tool result reuses the configured base url and aborts the previous stream', async () => {
  const original = (axiosBackendInstance as any).defaults.baseURL
  ;(axiosBackendInstance as any).defaults.baseURL = ''
  try {
    const { component } = render(ChatInterface as Component)
    await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

    await (component as any).sendMessage('first turn')
    expect(globalThis.fetch).toHaveBeenNthCalledWith(
      1,
      '/agent/chat/stream',
      expect.objectContaining({ method: 'POST' })
    )

    // free the loading lock so the tool result is accepted, which then has to
    // abort the still-open stream controller from the send above
    wsEventHandler({ type: 'done', conversation_id: 'conv-2' })
    await (component as any).submitToolResult('ask_human', 'ask-2', 'Sure')

    expect(globalThis.fetch).toHaveBeenNthCalledWith(
      2,
      '/agent/chat/stream',
      expect.objectContaining({ method: 'POST' })
    )
    expect(sentBody(1)).toEqual({
      message: '',
      conversation_id: 'conv-2',
      tool_result: {
        tool_name: 'ask_human',
        tool_call_id: 'ask-2',
        result: 'Sure'
      }
    })
  } finally {
    ;(axiosBackendInstance as any).defaults.baseURL = original
  }
})

test('an aborted tool-result request is not surfaced', async () => {
  ;(globalThis.fetch as any).mockRejectedValue(
    Object.assign(new Error('aborted'), { name: 'AbortError' })
  )
  const { component } = render(ChatInterface as Component)

  await (component as any).submitToolResult('ask_human', 'ask-9', 'Maybe')

  await waitFor(() => expect(screen.getByText('Maybe')).toBeTruthy())
  expect(errorText()).toBeUndefined()
  expect(stopButton()).toBeTruthy()
})

// ---------------------------------------------------------------------------
// stop generation
// ---------------------------------------------------------------------------

test('stopping generation asks the backend to cancel the conversation', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  // establish a conversation id first
  wsEventHandler({ type: 'done', conversation_id: 'conv-9' })

  await typeAndSend('long running question')
  await waitFor(() => expect(stopButton()).toBeTruthy())

  wsEventHandler({
    type: 'status',
    status: 'finalizing',
    message: 'Working...'
  })
  await waitFor(() => expect(screen.getByText('Working...')).toBeTruthy())

  await fireEvent.click(stopButton())

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/chat/conv-9/cancel')
  })
  await waitFor(() => {
    expect(screen.getByText('Generation stopped')).toBeTruthy()
  })
  expect(screen.queryByText('Working...')).toBeNull()
  expect(stopButton()).toBeNull()
})

test('cancelling with no status bubble on screen adds none', async () => {
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'done', conversation_id: 'conv-5' })
  await typeAndSend('question')
  await waitFor(() => expect(stopButton()).toBeTruthy())

  await fireEvent.click(stopButton())

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/chat/conv-5/cancel')
  })
  await waitFor(() => expect(stopButton()).toBeNull())
  expect(screen.queryByText('Generation stopped')).toBeNull()
  expect(document.querySelectorAll('.status-message')).toHaveLength(0)
})

test('a failing cancel request still stops the spinner locally', async () => {
  mockedAxios.post.mockRejectedValue(new Error('backend gone'))
  render(ChatInterface as Component)
  await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

  wsEventHandler({ type: 'done', conversation_id: 'conv-3' })
  await typeAndSend('please stop')
  await waitFor(() => expect(stopButton()).toBeTruthy())

  await fireEvent.click(stopButton())

  await waitFor(() => expect(stopButton()).toBeNull())
  expect(errorSpy).toHaveBeenCalledWith(
    'Failed to send explicit cancel signal to backend:',
    expect.any(Error)
  )
})

test('stopping before a conversation exists aborts locally without a cancel call', async () => {
  render(ChatInterface as Component)
  await typeAndSend('very first message')
  await waitFor(() => expect(stopButton()).toBeTruthy())

  await fireEvent.click(stopButton())

  await waitFor(() => expect(stopButton()).toBeNull())
  expect(mockedAxios.post).not.toHaveBeenCalled()
  expect(sendButton()).toBeTruthy()
})

// ---------------------------------------------------------------------------
// conversation history loading
// ---------------------------------------------------------------------------

test('history keeps pending ask_human calls and drops tool-call-only turns', async () => {
  const history = [
    { role: 'user', content: 'Do the thing' },
    {
      role: 'assistant',
      content: '',
      tool_calls: [
        {
          id: 'tc-1',
          function: {
            name: 'ask_human',
            arguments: JSON.stringify({
              question: 'Which one?',
              options: ['A', 'B']
            })
          }
        }
      ]
    },
    {
      role: 'assistant',
      content: '',
      tool_calls: [
        { id: 'tc-2', function: { name: 'calculator', arguments: '{}' } }
      ]
    },
    { role: 'tool', content: '42', tool_call_id: 'tc-2', name: 'calculator' },
    { role: 'user', content: null }
  ]

  mockedAxios.get.mockImplementation((url: string) => {
    if (url.includes('/messages')) return Promise.resolve({ data: history })
    if (url === 'agent/model-capabilities')
      return Promise.resolve({ data: { vision: false, audio: false } })
    return Promise.resolve({ data: {} })
  })

  render(ChatInterface as Component, {
    props: { currentConversationId: 'c-1' }
  })

  await waitFor(() => expect(screen.getByText('Do the thing')).toBeTruthy())
  // the unanswered ask_human tool call is rebuilt as an interactive prompt
  expect(screen.getByText('Which one?')).toBeTruthy()
  expect(screen.getByText('Waiting for your input...')).toBeTruthy()
  // the calculator tool call already has a result, so only the result shows
  expect(screen.getByText('42')).toBeTruthy()
  // neither empty tool-call-only assistant turn produced a blank bubble
  expect(screen.queryAllByText('Assistant')).toHaveLength(0)
  // a user turn with no content is still kept, just empty
  const userBubbles = document.querySelectorAll('.message.user')
  expect(userBubbles).toHaveLength(2)
  expect(
    userBubbles[1].querySelector('.message-content')?.textContent?.trim()
  ).toBe('')
})

test('a failing history request shows a load error', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url.includes('/messages'))
      return Promise.reject(new Error('no history'))
    return Promise.resolve({ data: {} })
  })

  render(ChatInterface as Component, {
    props: { currentConversationId: 'c-2' }
  })

  await waitFor(() =>
    expect(errorText()).toBe('Failed to load conversation history')
  )
  expect(errorSpy).toHaveBeenCalledWith(
    'Failed to load messages:',
    expect.any(Error)
  )
})

test('clearing the selected conversation empties the transcript', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url.includes('/messages'))
      return Promise.resolve({
        data: [{ role: 'user', content: 'Older question' }]
      })
    return Promise.resolve({ data: {} })
  })

  const { rerender } = render(ChatInterface as Component, {
    props: { currentConversationId: 'c-3' }
  })
  await waitFor(() => expect(screen.getByText('Older question')).toBeTruthy())

  await rerender({ currentConversationId: undefined })

  await waitFor(() => {
    expect(
      screen.getByText(/Start a conversation with the AI agent/)
    ).toBeTruthy()
  })
  expect(screen.queryByText('Older question')).toBeNull()
})

// ---------------------------------------------------------------------------
// auto-scroll
// ---------------------------------------------------------------------------

test('sending force-scrolls to the bottom even after the user scrolled up', async () => {
  render(ChatInterface as Component)
  const container = document.querySelector('.chat-messages') as HTMLDivElement
  const scrollTo = vi.fn()
  container.scrollTo = scrollTo
  Object.defineProperty(container, 'scrollHeight', {
    value: 1000,
    configurable: true
  })
  Object.defineProperty(container, 'clientHeight', {
    value: 400,
    configurable: true
  })
  Object.defineProperty(container, 'scrollTop', {
    value: 0,
    writable: true,
    configurable: true
  })

  // user is 600px away from the bottom
  await fireEvent.scroll(container)

  await typeAndSend('scroll me down')

  // the passive per-message autoscroll respects the user's position...
  await wait(60)
  expect(scrollTo).not.toHaveBeenCalled()

  // ...but sending forces a jump to the bottom
  await waitFor(() =>
    expect(scrollTo).toHaveBeenCalledWith({ top: 1000, behavior: 'auto' })
  )

  // the scroll event that the programmatic scroll itself triggers must not be
  // mistaken for the user scrolling away again
  await fireEvent.scroll(container)
  await wait(160)
  const forced = scrollTo.mock.calls.length

  wsEventHandler({
    type: 'status',
    status: 'finalizing',
    message: 'still here'
  })
  await wait(60)
  expect(scrollTo.mock.calls.length).toBeGreaterThan(forced)

  // a further forced scroll that really does land at the bottom keeps the
  // "at bottom" flag set once the animation settles
  wsEventHandler({ type: 'done' })
  container.scrollTop = 600
  await typeAndSend('and again')
  await wait(250)
  expect(scrollTo.mock.calls.length).toBeGreaterThan(forced + 1)
})

// ---------------------------------------------------------------------------
// resize handles
// ---------------------------------------------------------------------------

test('dragging the bottom handle resizes the height and respects the minimum', async () => {
  render(ChatInterface as Component)
  const iface = document.querySelector('.chat-interface') as HTMLElement
  const startHeight = parseInt(iface.style.height, 10)
  expect(iface.style.transition).toBe('height 0.2s ease, width 0.2s ease')

  await fireEvent.mouseDown(screen.getByLabelText('Resize chat height'), {
    clientX: 0,
    clientY: 200
  })
  // transitions are disabled while dragging so the handle tracks the cursor
  expect(iface.style.transition).toBe('none')

  await fireEvent.mouseMove(window, { clientX: 0, clientY: 300 })
  expect(iface.style.height).toBe(`${startHeight + 100}px`)

  // dragging past the 150px floor is ignored
  await fireEvent.mouseMove(window, { clientX: 0, clientY: 200 - startHeight })
  expect(iface.style.height).toBe(`${startHeight + 100}px`)

  await fireEvent.mouseUp(window)
  expect(iface.style.transition).toBe('height 0.2s ease, width 0.2s ease')

  // the move listener was detached, so later movement no longer resizes
  await fireEvent.mouseMove(window, { clientX: 0, clientY: 900 })
  expect(iface.style.height).toBe(`${startHeight + 100}px`)
})

test('dragging the right handle resizes the width within bounds', async () => {
  render(ChatInterface as Component)
  const iface = document.querySelector('.chat-interface') as HTMLElement
  const startWidth = parseInt(iface.style.width, 10)
  const startHeight = parseInt(iface.style.height, 10)

  await fireEvent.mouseDown(screen.getByLabelText('Resize chat width'), {
    clientX: 500,
    clientY: 0
  })

  await fireEvent.mouseMove(window, {
    clientX: 500 + (400 - startWidth),
    clientY: 0
  })
  expect(iface.style.width).toBe('400px')

  // narrower than 320px is rejected
  await fireEvent.mouseMove(window, {
    clientX: 500 + (300 - startWidth),
    clientY: 0
  })
  expect(iface.style.width).toBe('400px')

  // wider than the viewport is rejected
  await fireEvent.mouseMove(window, {
    clientX: 500 + (window.innerWidth - startWidth),
    clientY: 0
  })
  expect(iface.style.width).toBe('400px')

  // a horizontal drag never touches the height
  expect(iface.style.height).toBe(`${startHeight}px`)
  await fireEvent.mouseUp(window)
})

test('dragging the corner handle resizes both dimensions', async () => {
  render(ChatInterface as Component)
  const iface = document.querySelector('.chat-interface') as HTMLElement
  const startHeight = parseInt(iface.style.height, 10)
  const startWidth = parseInt(iface.style.width, 10)

  await fireEvent.mouseDown(
    screen.getByLabelText('Resize chat both directions'),
    { clientX: 500, clientY: 500 }
  )
  await fireEvent.mouseMove(window, {
    clientX: 500 + (420 - startWidth),
    clientY: 560
  })

  expect(iface.style.height).toBe(`${startHeight + 60}px`)
  expect(iface.style.width).toBe('420px')
  await fireEvent.mouseUp(window)
})

// ---------------------------------------------------------------------------
// text to speech
// ---------------------------------------------------------------------------

test('with TTS enabled a completed answer is spoken and can be stopped', async () => {
  const speak = vi.fn()
  const cancel = vi.fn()
  ;(window as any).speechSynthesis = { speak, cancel }
  ;(window as any).SpeechSynthesisUtterance = class {
    text: string
    rate = 1
    pitch = 1
    volume = 1
    lang = ''
    onstart: any = null
    onend: any = null
    onerror: any = null
    constructor(text: string) {
      this.text = text
    }
  }

  try {
    render(ChatInterface as Component)
    await waitFor(() => expect(typeof wsEventHandler).toBe('function'))

    await fireEvent.click(screen.getByTitle('Read Messages: Off'))
    await waitFor(() =>
      expect(screen.getByTitle('Read Messages: On')).toBeTruthy()
    )

    wsEventHandler({ type: 'text_chunk', text: '**Bold** answer 🚀' })
    wsEventHandler({ type: 'done' })

    await waitFor(() => expect(speak).toHaveBeenCalledTimes(1))
    // markdown and emoji are stripped before speaking
    expect(speak.mock.calls[0][0].text).toBe('Bold answer')
    expect(speak.mock.calls[0][0].lang).toBe('en-US')

    // speaking state is reflected in the control, which now stops playback
    const cancelsBefore = cancel.mock.calls.length
    await waitFor(() => expect(screen.getByTitle('Stop Speaking')).toBeTruthy())
    await fireEvent.click(screen.getByTitle('Stop Speaking'))
    expect(cancel.mock.calls.length).toBeGreaterThan(cancelsBefore)
    await waitFor(() =>
      expect(screen.getByTitle('Read Messages: On')).toBeTruthy()
    )

    // an answer that cleans down to nothing speakable is not spoken
    wsEventHandler({ type: 'text_chunk', text: '🚀' })
    wsEventHandler({ type: 'done' })
    await wait(30)
    expect(speak).toHaveBeenCalledTimes(1)
  } finally {
    delete (window as any).speechSynthesis
    delete (window as any).SpeechSynthesisUtterance
  }
})
