/**
 * @vitest-environment jsdom
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { useWebSocket, type WebSocketOptions } from './useWebSocket'

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
    // Simulate connection opening
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
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('creates WebSocket connection', async () => {
  const onOpen = vi.fn()
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onOpen
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  expect(onOpen).toHaveBeenCalledTimes(1)
  expect(ws.isConnected).toBe(true)
})

test('handles WebSocket messages', async () => {
  const onMessage = vi.fn()
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onMessage
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Simulate message
  const messageEvent = new MessageEvent('message', {
    data: JSON.stringify({ type: 'test', data: 'hello' })
  })
  ws.socket?.onmessage?.(messageEvent)

  expect(onMessage).toHaveBeenCalledWith(messageEvent)
})

test('handles WebSocket errors', async () => {
  const onError = vi.fn()
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onError
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Simulate error
  const errorEvent = new Event('error')
  ws.socket?.onerror?.(errorEvent)

  expect(onError).toHaveBeenCalledWith(errorEvent)
  expect(ws.isConnected).toBe(false)
})

test('handles WebSocket close', async () => {
  const onClose = vi.fn()
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onClose
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  ws.disconnect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  expect(onClose).toHaveBeenCalled()
  expect(ws.isConnected).toBe(false)
})

test('sends data when connected', async () => {
  const sendSpy = vi.spyOn(MockWebSocket.prototype, 'send')
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080'
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  const result = ws.send('test message')

  expect(result).toBe(true)
  expect(sendSpy).toHaveBeenCalledWith('test message')
})

test('fails to send when not connected', () => {
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080'
  }

  const ws = useWebSocket(options)
  // Don't connect

  const result = ws.send('test message')

  expect(result).toBe(false)
})

test('auto-reconnects on close by default', async () => {
  const onOpen = vi.fn()
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onOpen,
    reconnectInterval: 10
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))
  expect(onOpen).toHaveBeenCalledTimes(1)

  // Close connection
  ws.socket?.close()

  // Wait for reconnect
  await new Promise((resolve) => setTimeout(resolve, 50))

  expect(onOpen).toHaveBeenCalledTimes(2)
})

test('does not auto-reconnect when disabled', async () => {
  const onOpen = vi.fn()
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onOpen,
    autoReconnect: false,
    reconnectInterval: 10
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))
  expect(onOpen).toHaveBeenCalledTimes(1)

  // Close connection
  ws.socket?.close()

  // Wait - should not reconnect
  await new Promise((resolve) => setTimeout(resolve, 20))

  expect(onOpen).toHaveBeenCalledTimes(1)
})

test('disconnect clears reconnect timeout and closes connection', async () => {
  const onOpen = vi.fn()
  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onOpen,
    reconnectInterval: 100
  }

  const ws = useWebSocket(options)
  ws.connect()

  await new Promise((resolve) => setTimeout(resolve, 10))

  // Verify connected
  expect(ws.isConnected).toBe(true)
  expect(onOpen).toHaveBeenCalledTimes(1)

  // Disconnect
  ws.disconnect()

  // Immediately after disconnect, socket should be null
  expect(ws.socket).toBe(null)

  // Wait a bit for any async operations
  await new Promise((resolve) => setTimeout(resolve, 50))

  // After disconnect, should not be connected
  expect(ws.isConnected).toBe(false)
})

test('handles connection errors gracefully', () => {
  const onError = vi.fn()
  const originalWebSocket = global.WebSocket

  // Make WebSocket constructor throw
  global.WebSocket = vi.fn(() => {
    throw new Error('Connection failed')
  }) as any

  const options: WebSocketOptions = {
    url: 'ws://localhost:8080',
    onError
  }

  const ws = useWebSocket(options)
  ws.connect()

  expect(onError).toHaveBeenCalled()
  expect(ws.isConnected).toBe(false)

  // Restore
  global.WebSocket = originalWebSocket
})
