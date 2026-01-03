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

vi.mock('@hooks/useAgentWebSocket', () => ({
  useAgentWebSocket: vi.fn((handler) => {
    wsEventHandler = handler
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

beforeEach(() => {
  vi.clearAllMocks()
  clearToolsCache() // Reset tool cache
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})

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
