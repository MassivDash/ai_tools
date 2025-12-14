/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, waitFor, cleanup } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import Terminal from './terminal.svelte'

// Mock WebSocket
class MockWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  readyState = MockWebSocket.CONNECTING
  onopen: ((event: Event) => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  onclose: ((event: CloseEvent) => void) | null = null
  private _timeoutId: ReturnType<typeof setTimeout> | null = null

  constructor(public url: string) {
    this._timeoutId = setTimeout(() => {
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
    if (this._timeoutId) {
      clearTimeout(this._timeoutId)
      this._timeoutId = null
    }
    if (this.readyState !== MockWebSocket.CLOSED) {
      this.readyState = MockWebSocket.CLOSED
      if (this.onclose) {
        this.onclose(new CloseEvent('close'))
      }
    }
  }

  addEventListener() {
    // Mock addEventListener
  }

  removeEventListener() {
    // Mock removeEventListener
  }
}

beforeEach(() => {
  // Ensure WebSocket is properly mocked as a constructor function
  const WebSocketMock = function (this: any, url: string) {
    return new MockWebSocket(url)
  } as any

  // Copy static properties
  WebSocketMock.CONNECTING = MockWebSocket.CONNECTING
  WebSocketMock.OPEN = MockWebSocket.OPEN
  WebSocketMock.CLOSING = MockWebSocket.CLOSING
  WebSocketMock.CLOSED = MockWebSocket.CLOSED

  global.WebSocket = WebSocketMock
  // Also set it on window for good measure
  if (typeof window !== 'undefined') {
    ;(window as any).WebSocket = WebSocketMock
  }
  vi.spyOn(global, 'setTimeout')
  vi.spyOn(global, 'clearTimeout')
})

afterEach(async () => {
  cleanup()
  // Wait for cleanup to complete
  await new Promise((resolve) => setTimeout(resolve, 50))
  vi.restoreAllMocks()
})

test('renders terminal component', async () => {
  const { unmount } = render(Terminal)

  expect(screen.getByText('Server Output')).toBeTruthy()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('shows disconnected status initially', async () => {
  const { unmount } = render(Terminal)

  expect(screen.getByTitle('Disconnected')).toBeTruthy()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('connects to WebSocket on mount', async () => {
  const wsSpy = vi.spyOn(global, 'WebSocket')

  const { unmount } = render(Terminal)

  await waitFor(() => {
    expect(wsSpy).toHaveBeenCalled()
  })

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('shows empty logs message initially', async () => {
  const { unmount } = render(Terminal)

  expect(
    screen.getByText('No logs yet. Start the server to see output.')
  ).toBeTruthy()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('renders terminal structure correctly', async () => {
  const { container, unmount } = render(Terminal)

  expect(container.querySelector('.terminal-container')).toBeTruthy()
  expect(container.querySelector('.terminal-header')).toBeTruthy()
  expect(container.querySelector('.terminal-content')).toBeTruthy()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})
