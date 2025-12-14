/**
 * @vitest-environment jsdom
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { useStatusWebSocket } from './useStatusWebSocket'

// Mock WebSocket
class MockWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  url: string
  readyState: number = MockWebSocket.CONNECTING
  onopen: ((_event: Event) => void) | null = null
  onmessage: ((_event: MessageEvent) => void) | null = null
  onerror: ((_event: Event) => void) | null = null
  onclose: ((_event: CloseEvent) => void) | null = null

  constructor(url: string) {
    this.url = url
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN
      if (this.onopen) {
        this.onopen(new Event('open'))
      }
    }, 0)
  }

  send(_data: string | ArrayBuffer | Blob) {
    // Mock send
  }

  close(_code?: number, _reason?: string) {
    this.readyState = MockWebSocket.CLOSING
    setTimeout(() => {
      this.readyState = MockWebSocket.CLOSED
      if (this.onclose) {
        this.onclose(new CloseEvent('close'))
      }
    }, 0)
  }

  addEventListener() {
    // Mock addEventListener
  }

  removeEventListener() {
    // Mock removeEventListener
  }
}

beforeEach(() => {
  global.WebSocket = MockWebSocket as any
  window.WebSocket = MockWebSocket as any
  vi.spyOn(global, 'setTimeout')
  vi.spyOn(global, 'clearTimeout')
  vi.spyOn(console, 'error').mockImplementation(() => {})

  // Mock import.meta.env
  Object.defineProperty(import.meta, 'env', {
    value: { PUBLIC_API_URL: 'http://localhost:3000' },
    writable: true
  })
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('connects to status WebSocket', async () => {
  const onStatusChange = vi.fn()
  const ws = useStatusWebSocket(onStatusChange)

  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  expect(ws.isConnected).toBe(true)
})

test('handles status messages', async () => {
  const onStatusChange = vi.fn()
  const ws = useStatusWebSocket(onStatusChange)

  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Simulate status message
  const statusMessage = {
    type: 'status',
    active: true,
    port: 8080
  }
  const messageEvent = new MessageEvent('message', {
    data: JSON.stringify(statusMessage)
  })
  ws.socket?.onmessage?.(messageEvent)

  expect(onStatusChange).toHaveBeenCalledWith({
    active: true,
    port: 8080
  })
})

test('calls onStatusChange only when status changes', async () => {
  const onStatusChange = vi.fn()
  const ws = useStatusWebSocket(onStatusChange)

  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // First status
  const status1 = {
    type: 'status',
    active: true,
    port: 8080
  }
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify(status1)
    })
  )

  expect(onStatusChange).toHaveBeenCalledTimes(1)

  // Same status - should not call again
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify(status1)
    })
  )

  expect(onStatusChange).toHaveBeenCalledTimes(1)

  // Different status - should call again
  const status2 = {
    type: 'status',
    active: false,
    port: 8080
  }
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify(status2)
    })
  )

  expect(onStatusChange).toHaveBeenCalledTimes(2)
  expect(onStatusChange).toHaveBeenLastCalledWith({
    active: false,
    port: 8080
  })
})

test('calls onServerReady when server becomes active', async () => {
  const onStatusChange = vi.fn()
  const onServerReady = vi.fn()
  const ws = useStatusWebSocket(onStatusChange, onServerReady)

  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Server inactive
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify({ type: 'status', active: false, port: 8080 })
    })
  )

  expect(onServerReady).not.toHaveBeenCalled()

  // Server becomes active
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify({ type: 'status', active: true, port: 8080 })
    })
  )

  expect(onServerReady).toHaveBeenCalledTimes(1)
})

test('does not call onServerReady if server was already active', async () => {
  const onStatusChange = vi.fn()
  const onServerReady = vi.fn()
  const ws = useStatusWebSocket(onStatusChange, onServerReady)

  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Server already active
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify({ type: 'status', active: true, port: 8080 })
    })
  )

  expect(onServerReady).not.toHaveBeenCalled()
})

test('handles invalid JSON messages gracefully', async () => {
  const onStatusChange = vi.fn()
  const ws = useStatusWebSocket(onStatusChange)

  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Invalid JSON
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: 'invalid json'
    })
  )

  expect(onStatusChange).not.toHaveBeenCalled()
  expect(console.error).toHaveBeenCalled()
})

test('ignores non-status messages', async () => {
  const onStatusChange = vi.fn()
  const ws = useStatusWebSocket(onStatusChange)

  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Non-status message
  ws.socket?.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify({ type: 'other', data: 'test' })
    })
  )

  expect(onStatusChange).not.toHaveBeenCalled()
})

test('constructs WebSocket URL', () => {
  const onStatusChange = vi.fn()
  const ws = useStatusWebSocket(onStatusChange)

  ws.connect()

  // Just verify that a URL was constructed (exact URL depends on environment)
  expect(ws.socket?.url).toBeTruthy()
  expect(ws.socket?.url).toContain('/api/llama-server/status/ws')
})

test('constructs WebSocket URL with correct protocol', () => {
  const onStatusChange = vi.fn()
  const ws = useStatusWebSocket(onStatusChange)

  ws.connect()

  // Verify protocol is ws or wss
  const url = ws.socket?.url || ''
  expect(url.startsWith('ws://') || url.startsWith('wss://')).toBe(true)
})
