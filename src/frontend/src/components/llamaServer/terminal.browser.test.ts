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
  onopen: ((_event: Event) => void) | null = null
  onmessage: ((_event: MessageEvent) => void) | null = null
  onerror: ((_event: Event) => void) | null = null
  onclose: ((_event: CloseEvent) => void) | null = null
  private _timeoutId: ReturnType<typeof setTimeout> | null = null

  constructor(public _url: string) {
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

const wsInstances: MockWebSocket[] = []

const lastSocket = async (): Promise<MockWebSocket> => {
  await waitFor(() => {
    expect(wsInstances.length).toBeGreaterThan(0)
  })
  return wsInstances[wsInstances.length - 1]
}

// The component schedules a 10ms auto-scroll timeout per message and never
// clears it on destroy, so let it run before unmounting.
const settleAutoScroll = () => new Promise((resolve) => setTimeout(resolve, 30))

const logLine = (
  line: string,
  source: 'stdout' | 'stderr' = 'stdout',
  timestamp = 1700000000
) => ({ timestamp, line, source })

beforeEach(() => {
  wsInstances.length = 0
  vi.spyOn(console, 'error').mockImplementation(() => {})

  // Ensure WebSocket is properly mocked as a constructor function
  const WebSocketMock = function (this: any, url: string) {
    const instance = new MockWebSocket(url)
    wsInstances.push(instance)
    return instance
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
  vi.unstubAllEnvs()
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

test('builds a secure ws url from PUBLIC_API_URL, dropping the /api suffix', async () => {
  vi.stubEnv('PUBLIC_API_URL', 'https://llm.example.com/api/')

  const { unmount } = render(Terminal)

  const socket = await lastSocket()
  expect(socket._url).toBe('wss://llm.example.com/api/llama-server/logs/ws')

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('falls back to the page origin when PUBLIC_API_URL is not set', async () => {
  vi.stubEnv('PUBLIC_API_URL', '')

  const { unmount } = render(Terminal)

  const socket = await lastSocket()
  expect(socket._url).toBe(
    `ws://${window.location.host}/api/llama-server/logs/ws`
  )

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('switches the indicator to connected once the socket opens', async () => {
  const { unmount } = render(Terminal)

  await waitFor(() => {
    expect(screen.getByTitle('Connected')).toBeTruthy()
  })
  expect(screen.queryByTitle('Disconnected')).not.toBeInTheDocument()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('appends streamed log lines with their timestamp and source', async () => {
  const { container, unmount } = render(Terminal)
  const socket = await lastSocket()

  socket.onmessage!({
    data: JSON.stringify({ type: 'log', log: logLine('loading model') })
  } as MessageEvent)
  socket.onmessage!({
    data: JSON.stringify({
      type: 'log',
      log: logLine('cuda not found', 'stderr', 1700000060)
    })
  } as MessageEvent)

  await waitFor(() => {
    expect(container.querySelectorAll('.log-line')).toHaveLength(2)
  })

  expect(
    screen.queryByText('No logs yet. Start the server to see output.')
  ).not.toBeInTheDocument()

  const lines = container.querySelectorAll('.log-line')
  expect(lines[0]).toHaveClass('stdout')
  expect(lines[0].querySelector('.log-source')).toHaveTextContent('[stdout]')
  expect(lines[0].querySelector('.log-text')).toHaveTextContent('loading model')
  expect(lines[0].querySelector('.log-timestamp')).toHaveTextContent(
    new Date(1700000000 * 1000).toLocaleTimeString()
  )

  expect(lines[1]).toHaveClass('stderr')
  expect(lines[1].querySelector('.log-source')).toHaveTextContent('[stderr]')
  expect(lines[1].querySelector('.log-text')).toHaveTextContent(
    'cuda not found'
  )
  expect(lines[1].querySelector('.log-timestamp')).toHaveTextContent(
    new Date(1700000060 * 1000).toLocaleTimeString()
  )

  await settleAutoScroll()
  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('auto-scrolls the log pane to the bottom for new lines and batches', async () => {
  const { container, unmount } = render(Terminal)
  const socket = await lastSocket()
  const pane = container.querySelector('.terminal-content') as HTMLElement
  Object.defineProperty(pane, 'scrollHeight', {
    value: 500,
    configurable: true
  })

  socket.onmessage!({
    data: JSON.stringify({ type: 'log', log: logLine('first line') })
  } as MessageEvent)

  await waitFor(() => {
    expect(pane.scrollTop).toBe(500)
  })

  pane.scrollTop = 0
  Object.defineProperty(pane, 'scrollHeight', {
    value: 900,
    configurable: true
  })
  socket.onmessage!({
    data: JSON.stringify({
      type: 'logs_batch',
      logs: [logLine('batched line')]
    })
  } as MessageEvent)

  await waitFor(() => {
    expect(pane.scrollTop).toBe(900)
  })

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('a logs_batch message replaces the whole buffer', async () => {
  const { container, unmount } = render(Terminal)
  const socket = await lastSocket()

  socket.onmessage!({
    data: JSON.stringify({ type: 'log', log: logLine('stale line') })
  } as MessageEvent)

  await waitFor(() => {
    expect(screen.getByText('stale line')).toBeTruthy()
  })

  socket.onmessage!({
    data: JSON.stringify({
      type: 'logs_batch',
      logs: [logLine('replay one'), logLine('replay two', 'stderr')]
    })
  } as MessageEvent)

  await waitFor(() => {
    expect(container.querySelectorAll('.log-line')).toHaveLength(2)
  })
  expect(screen.queryByText('stale line')).not.toBeInTheDocument()
  expect(screen.getByText('replay one')).toBeTruthy()
  expect(screen.getByText('replay two')).toBeTruthy()

  await settleAutoScroll()
  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('ignores unknown message types', async () => {
  const { container, unmount } = render(Terminal)
  const socket = await lastSocket()

  socket.onmessage!({
    data: JSON.stringify({ type: 'status', active: true })
  } as MessageEvent)

  await new Promise((resolve) => setTimeout(resolve, 20))

  expect(container.querySelectorAll('.log-line')).toHaveLength(0)
  expect(
    screen.getByText('No logs yet. Start the server to see output.')
  ).toBeTruthy()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('logs a parse failure and keeps the existing buffer', async () => {
  const { container, unmount } = render(Terminal)
  const socket = await lastSocket()

  socket.onmessage!({ data: 'not-json' } as MessageEvent)

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to parse WebSocket message:',
      expect.any(Error)
    )
  })
  expect(container.querySelectorAll('.log-line')).toHaveLength(0)
  expect(
    screen.getByText('No logs yet. Start the server to see output.')
  ).toBeTruthy()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('marks the terminal disconnected when the socket errors', async () => {
  const { unmount } = render(Terminal)
  const socket = await lastSocket()

  await waitFor(() => {
    expect(screen.getByTitle('Connected')).toBeTruthy()
  })

  const errorEvent = new Event('error')
  socket.onerror!(errorEvent)

  await waitFor(() => {
    expect(screen.getByTitle('Disconnected')).toBeTruthy()
  })
  expect(console.error).toHaveBeenCalledWith(
    'Logs WebSocket error:',
    errorEvent
  )

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('schedules a reconnect two seconds after the socket closes', async () => {
  const { unmount } = render(Terminal)
  const socket = await lastSocket()

  await waitFor(() => {
    expect(screen.getByTitle('Connected')).toBeTruthy()
  })

  socket.close()

  await waitFor(() => {
    expect(screen.getByTitle('Disconnected')).toBeTruthy()
  })

  const reconnectCall = (
    setTimeout as unknown as ReturnType<typeof vi.fn>
  ).mock.calls.find((call: any[]) => call[1] === 2000)
  expect(reconnectCall).toBeDefined()

  // run the scheduled reconnect: a brand new socket must be opened
  reconnectCall![0]()

  await waitFor(() => {
    expect(wsInstances.length).toBe(2)
  })
  await waitFor(() => {
    expect(screen.getByTitle('Connected')).toBeTruthy()
  })

  // closing again clears the pending reconnect timer before rescheduling
  wsInstances[1].close()
  await waitFor(() => {
    expect(clearTimeout).toHaveBeenCalled()
  })

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('reports a connection attempt that throws instead of crashing', async () => {
  global.WebSocket = function () {
    throw new Error('ws unavailable')
  } as any

  const { unmount } = render(Terminal)

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to connect WebSocket:',
      expect.any(Error)
    )
  })
  expect(screen.getByTitle('Disconnected')).toBeTruthy()
  expect(
    screen.getByText('No logs yet. Start the server to see output.')
  ).toBeTruthy()

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))
})

test('closes the socket on unmount', async () => {
  const { unmount } = render(Terminal)
  const socket = await lastSocket()
  const closeSpy = vi.spyOn(socket, 'close')

  unmount()
  await new Promise((resolve) => setTimeout(resolve, 10))

  expect(closeSpy).toHaveBeenCalled()
  expect(socket.readyState).toBe(3)
})
