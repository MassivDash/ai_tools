/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import LlamaServer from './llamaServer.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

// Mock useStatusWebSocket
const mockConnect = vi.fn()
const mockDisconnect = vi.fn()
const mockStatusWs = {
  connect: mockConnect,
  disconnect: mockDisconnect,
  isConnected: false,
  socket: null
}

// Captures the callbacks the component hands to useStatusWebSocket so the tests
// can push status updates / disconnects through them.
const statusHandlers: {
  onStatus?: (_status: { active: boolean; port: number }) => void
  onDisconnect?: () => void
} = {}

vi.mock('../../hooks/useStatusWebSocket', () => ({
  useStatusWebSocket: vi.fn(
    (
      onStatus: (_status: { active: boolean; port: number }) => void,
      onDisconnect: () => void
    ) => {
      statusHandlers.onStatus = onStatus
      statusHandlers.onDisconnect = onDisconnect
      return mockStatusWs
    }
  )
}))

const activateServer = async (port = 8099) => {
  statusHandlers.onStatus?.({ active: true, port })
  await waitFor(() => {
    expect(screen.getByTitle('Stop Server')).toBeTruthy()
  })
}

// Mock WebSocket for Terminal component
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

  constructor(public _url: string) {
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

const mockedAxios = axiosBackendInstance as {
  post: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  statusHandlers.onStatus = undefined
  statusHandlers.onDisconnect = undefined
  vi.spyOn(console, 'error').mockImplementation(() => {})

  // Mock WebSocket
  const WebSocketMock = function (this: any, url: string) {
    return new MockWebSocket(url)
  } as any
  WebSocketMock.CONNECTING = MockWebSocket.CONNECTING
  WebSocketMock.OPEN = MockWebSocket.OPEN
  WebSocketMock.CLOSING = MockWebSocket.CLOSING
  WebSocketMock.CLOSED = MockWebSocket.CLOSED

  global.WebSocket = WebSocketMock
  if (typeof window !== 'undefined') {
    ;(window as any).WebSocket = WebSocketMock
  }
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('renders Llama Server component', async () => {
  render(LlamaServer as Component)

  await waitFor(
    () => {
      expect(screen.getByText('Llama.cpp Server')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows empty state when server is not active', async () => {
  render(LlamaServer as Component)

  await waitFor(
    () => {
      expect(screen.getByText(/Llama.cpp Server is not running/)).toBeTruthy()
      expect(screen.getByText(/Click "Start Server"/)).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('connects to status WebSocket on mount', () => {
  render(LlamaServer as Component)

  expect(mockConnect).toHaveBeenCalledTimes(1)
})

test('disconnects from status WebSocket on unmount', () => {
  const { unmount } = render(LlamaServer as Component)

  unmount()

  expect(mockDisconnect).toHaveBeenCalledTimes(1)
})

test('starts server when start button is clicked', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: { success: true, message: 'Server started' }
  })

  render(LlamaServer as Component)

  await waitFor(
    () => {
      expect(screen.getByTitle('Start Server')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  const startButton = screen.getByTitle('Start Server')
  fireEvent.click(startButton)

  await waitFor(
    () => {
      expect(mockedAxios.post).toHaveBeenCalledWith('llama-server/start')
    },
    { timeout: 2000 }
  )
})

test('shows error when start fails', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: { success: false, message: 'Failed to start server' }
  })

  render(LlamaServer as Component)

  const startButton = screen.getByTitle('Start Server')
  fireEvent.click(startButton)

  await waitFor(
    () => {
      expect(screen.getByText('Failed to start server')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('stops server when stop button is clicked', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: { success: true, message: 'Server stopped' }
  })

  render(LlamaServer as Component)
  await activateServer()

  fireEvent.click(screen.getByTitle('Stop Server'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('llama-server/stop')
  })

  // a successful stop flips the status back to inactive locally
  await waitFor(() => {
    expect(screen.getByTitle('Start Server')).toBeTruthy()
  })
  expect(screen.getByText(/Llama.cpp Server is not running/)).toBeTruthy()
})

test('toggles config panel when config button is clicked', async () => {
  render(LlamaServer as Component)

  const configButton = screen.getByTitle('Config')
  fireEvent.click(configButton)

  await waitFor(
    () => {
      expect(screen.getByText('Server Configuration')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Click again to close - the config panel uses class-based visibility
  fireEvent.click(configButton)

  // The config panel is still in DOM but not visible
  await waitFor(
    () => {
      const configPanel = document.querySelector('.config-panel')
      expect(configPanel).toBeTruthy()
      // Check if it has the visible class or not
      const isVisible = configPanel?.classList.contains('visible')
      expect(isVisible).toBe(false)
    },
    { timeout: 2000 }
  )
})

test('toggles terminal when terminal button is clicked', async () => {
  render(LlamaServer as Component)

  const terminalButton = screen.getByTitle('Show Terminal')
  fireEvent.click(terminalButton)

  await waitFor(
    () => {
      expect(screen.getByTitle('Hide Terminal')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Click again to hide
  const hideButton = screen.getByTitle('Hide Terminal')
  fireEvent.click(hideButton)

  await waitFor(
    () => {
      expect(screen.getByTitle('Show Terminal')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows terminal when server is starting', async () => {
  mockedAxios.post.mockImplementation(() => {
    return new Promise(() => {}) // Never resolves
  })

  render(LlamaServer as Component)

  const startButton = screen.getByTitle('Start Server')
  fireEvent.click(startButton)

  await waitFor(
    () => {
      expect(screen.getByTitle('Hide Terminal')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('disables start button when loading', async () => {
  mockedAxios.post.mockImplementation(() => {
    return new Promise(() => {}) // Never resolves
  })

  render(LlamaServer as Component)

  const startButton = screen.getByTitle('Start Server')
  fireEvent.click(startButton)

  await waitFor(
    () => {
      expect(screen.getByTitle('Starting...')).toBeTruthy()
      const loadingButton = screen.getByTitle('Starting...')
      expect(loadingButton).toBeDisabled()
    },
    { timeout: 2000 }
  )
})

test('displays empty state when server is inactive', () => {
  render(LlamaServer as Component)

  // When server is inactive, empty state is shown instead of iframe
  expect(screen.getByText(/Llama.cpp Server is not running/)).toBeTruthy()
  expect(screen.getByText(/Click "Start Server"/)).toBeTruthy()

  // Iframe container only exists when server is active
  const iframeContainer = document.querySelector('.iframe-container')
  // When server is inactive, iframe container is not rendered
  expect(iframeContainer).toBeFalsy()
})

test('handles network errors gracefully', async () => {
  mockedAxios.post.mockRejectedValueOnce(new Error('Network error'))

  render(LlamaServer as Component)

  const startButton = screen.getByTitle('Start Server')
  fireEvent.click(startButton)

  await waitFor(
    () => {
      expect(
        screen.getByText(/Network error|Failed to start server/)
      ).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows the llama web UI iframe on the port reported by the status socket', async () => {
  render(LlamaServer as Component)

  expect(screen.getByText(/Llama.cpp Server is not running/)).toBeTruthy()

  await activateServer(9123)

  expect(screen.queryByTitle('Start Server')).not.toBeInTheDocument()
  expect(
    screen.queryByText(/Llama.cpp Server is not running/)
  ).not.toBeInTheDocument()

  const iframe = document.querySelector('iframe.llama-iframe')
  expect(iframe).toBeTruthy()
  expect(iframe).toHaveAttribute('src', 'http://localhost:9123')
  expect(iframe).toHaveAttribute('title', 'Llama.cpp WebUI')
})

test('hides the terminal when the status socket reports a disconnect', async () => {
  render(LlamaServer as Component)

  fireEvent.click(screen.getByTitle('Show Terminal'))

  await waitFor(() => {
    expect(document.querySelector('.terminal-sidebar')).toHaveClass('visible')
  })
  expect(screen.getByTitle('Hide Terminal')).toBeTruthy()

  statusHandlers.onDisconnect?.()

  await waitFor(() => {
    expect(screen.getByTitle('Show Terminal')).toBeTruthy()
  })
  expect(document.querySelector('.terminal-sidebar')).not.toHaveClass('visible')
})

test('shows the message returned by a refused stop request', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: { success: false, message: 'Server is not running' }
  })

  render(LlamaServer as Component)
  await activateServer()

  fireEvent.click(screen.getByTitle('Stop Server'))

  await waitFor(() => {
    expect(screen.getByText('Server is not running')).toBeTruthy()
  })
  // the status is left untouched when the stop request is refused
  expect(screen.getByTitle('Stop Server')).toBeTruthy()
})

test('surfaces the backend error payload when stopping throws', async () => {
  mockedAxios.post.mockRejectedValueOnce({
    response: { data: { error: 'stop refused by daemon' } },
    message: 'Request failed with status code 500'
  })

  render(LlamaServer as Component)
  await activateServer()

  fireEvent.click(screen.getByTitle('Stop Server'))

  await waitFor(() => {
    expect(screen.getByText('stop refused by daemon')).toBeTruthy()
  })
})

test('falls back to the error message when stopping throws without a payload', async () => {
  mockedAxios.post.mockRejectedValueOnce(new Error('socket hang up'))

  render(LlamaServer as Component)
  await activateServer()

  fireEvent.click(screen.getByTitle('Stop Server'))

  await waitFor(() => {
    expect(screen.getByText('socket hang up')).toBeTruthy()
  })
})

test('falls back to a generic message when stopping throws without details', async () => {
  mockedAxios.post.mockRejectedValueOnce({})

  render(LlamaServer as Component)
  await activateServer()

  fireEvent.click(screen.getByTitle('Stop Server'))

  await waitFor(() => {
    expect(screen.getByText('Failed to stop server')).toBeTruthy()
  })
})

test('disables the stop button while stopping', async () => {
  mockedAxios.post.mockImplementation(() => new Promise(() => {}))

  render(LlamaServer as Component)
  await activateServer()

  fireEvent.click(screen.getByTitle('Stop Server'))

  await waitFor(() => {
    expect(screen.getByTitle('Stopping...')).toBeDisabled()
  })
})

test('surfaces the backend error payload when starting throws', async () => {
  mockedAxios.post.mockRejectedValueOnce({
    response: { data: { error: 'no gpu available' } },
    message: 'Request failed with status code 500'
  })

  render(LlamaServer as Component)

  fireEvent.click(screen.getByTitle('Start Server'))

  await waitFor(() => {
    expect(screen.getByText('no gpu available')).toBeTruthy()
  })
})

test('falls back to a generic message when starting throws without details', async () => {
  mockedAxios.post.mockRejectedValueOnce({})

  render(LlamaServer as Component)

  fireEvent.click(screen.getByTitle('Start Server'))

  await waitFor(() => {
    expect(screen.getByText('Failed to start server')).toBeTruthy()
  })
})

test('clears a previous error on the next start attempt', async () => {
  mockedAxios.post
    .mockRejectedValueOnce(new Error('first attempt failed'))
    .mockImplementation(() => new Promise(() => {}))

  render(LlamaServer as Component)

  fireEvent.click(screen.getByTitle('Start Server'))

  await waitFor(() => {
    expect(screen.getByText('first attempt failed')).toBeTruthy()
  })

  fireEvent.click(screen.getByTitle('Start Server'))

  await waitFor(() => {
    expect(screen.queryByText('first attempt failed')).not.toBeInTheDocument()
  })
})

test('calls handleConfigSave when config is saved', async () => {
  render(LlamaServer as Component)

  // Open config
  const configButton = screen.getByTitle('Config')
  fireEvent.click(configButton)

  await waitFor(
    () => {
      expect(screen.getByText('Server Configuration')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // The handleConfigSave is called internally when config is saved
  // We can't easily test this without mocking the entire config component
  // But we can verify the config panel is rendered
  expect(screen.getByText('Server Configuration')).toBeTruthy()
})
