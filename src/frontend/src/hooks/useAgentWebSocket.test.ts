/**
 * @vitest-environment jsdom
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { useAgentWebSocket, type AgentStreamEvent } from './useAgentWebSocket'

// Minimal hand-rolled socket, mirroring the MockWebSocket style in useWebSocket.test.ts,
// but with instance tracking so the constructed url and pushed frames can be asserted.
const sockets: MockWebSocket[] = []

class MockWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  url: string
  readyState = MockWebSocket.CONNECTING
  onopen: ((_e: Event) => void) | null = null
  onmessage: ((_e: MessageEvent) => void) | null = null
  onerror: ((_e: Event) => void) | null = null
  onclose: ((_e: CloseEvent) => void) | null = null
  sent: unknown[] = []

  constructor(url: string) {
    this.url = url
    sockets.push(this)
  }

  open() {
    this.readyState = MockWebSocket.OPEN
    this.onopen?.(new Event('open'))
  }

  emit(data: string) {
    this.onmessage?.({ data } as MessageEvent)
  }

  send(data: unknown) {
    this.sent.push(data)
  }

  close() {
    this.readyState = MockWebSocket.CLOSED
  }
}

beforeEach(() => {
  sockets.length = 0
  global.WebSocket = MockWebSocket as unknown as typeof WebSocket
  window.WebSocket = MockWebSocket as unknown as typeof WebSocket
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.unstubAllEnvs()
  vi.restoreAllMocks()
})

test('derives an ws:// url from a plain http PUBLIC_API_URL', () => {
  vi.stubEnv('PUBLIC_API_URL', 'http://localhost:8000')

  useAgentWebSocket(() => {}).connect()

  expect(sockets[0].url).toBe('ws://localhost:8000/api/agent/stream/ws')
})

test('derives a wss:// url from an https PUBLIC_API_URL', () => {
  vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com')

  useAgentWebSocket(() => {}).connect()

  expect(sockets[0].url).toBe('wss://api.example.com/api/agent/stream/ws')
})

test('strips a trailing /api segment before appending the ws path', () => {
  vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com/api')

  useAgentWebSocket(() => {}).connect()

  expect(sockets[0].url).toBe('wss://api.example.com/api/agent/stream/ws')
})

test('strips a trailing /api/ segment (with slash) too', () => {
  vi.stubEnv('PUBLIC_API_URL', 'http://localhost:8000/api/')

  useAgentWebSocket(() => {}).connect()

  expect(sockets[0].url).toBe('ws://localhost:8000/api/agent/stream/ws')
})

test('strips a bare trailing slash', () => {
  vi.stubEnv('PUBLIC_API_URL', 'http://localhost:8000/')

  useAgentWebSocket(() => {}).connect()

  expect(sockets[0].url).toBe('ws://localhost:8000/api/agent/stream/ws')
})

test('falls back to window.location.origin when PUBLIC_API_URL is unset', () => {
  vi.stubEnv('PUBLIC_API_URL', '')

  useAgentWebSocket(() => {}).connect()

  const expectedHost = window.location.origin.replace(/^https?:\/\//, '')
  expect(sockets[0].url).toBe(`ws://${expectedHost}/api/agent/stream/ws`)
})

test('parses an incoming frame and forwards it to the event callback', () => {
  const onEvent = vi.fn<(_e: AgentStreamEvent) => void>()
  useAgentWebSocket(onEvent).connect()
  sockets[0].open()

  sockets[0].emit(JSON.stringify({ type: 'text_chunk', text: 'hello there' }))

  expect(onEvent).toHaveBeenCalledTimes(1)
  expect(onEvent).toHaveBeenCalledWith({
    type: 'text_chunk',
    text: 'hello there'
  })
})

test('forwards each of the agent event shapes verbatim', () => {
  const received: AgentStreamEvent[] = []
  useAgentWebSocket((e) => received.push(e)).connect()
  sockets[0].open()

  const frames: AgentStreamEvent[] = [
    { type: 'status', status: 'thinking' },
    { type: 'tool_call', tool_name: 'calculator', arguments: '{"a":1}' },
    {
      type: 'tool_result',
      tool_name: 'calculator',
      success: true,
      result: '2'
    },
    { type: 'text_chunk', text: 'partial' },
    {
      type: 'done',
      conversation_id: 'conv-7',
      tool_calls: [{ tool_name: 'calculator', result: '2' }]
    },
    { type: 'error', message: 'upstream exploded' }
  ]
  frames.forEach((f) => sockets[0].emit(JSON.stringify(f)))

  expect(received).toEqual(frames)
})

test('a malformed frame is logged and does not invoke the event callback or throw', () => {
  const onEvent = vi.fn()
  const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  useAgentWebSocket(onEvent).connect()
  sockets[0].open()

  expect(() => sockets[0].emit('{ not json')).not.toThrow()

  expect(onEvent).not.toHaveBeenCalled()
  expect(errorSpy).toHaveBeenCalledWith(
    'Failed to parse agent WebSocket message:',
    expect.any(Error)
  )
})

test('a malformed frame does not stop later valid frames being delivered', () => {
  const onEvent = vi.fn()
  useAgentWebSocket(onEvent).connect()
  sockets[0].open()

  sockets[0].emit('<<garbage>>')
  sockets[0].emit(JSON.stringify({ type: 'done', conversation_id: 'c1' }))

  expect(onEvent).toHaveBeenCalledTimes(1)
  expect(onEvent).toHaveBeenCalledWith({ type: 'done', conversation_id: 'c1' })
})

test('socket errors are forwarded to the optional error callback', () => {
  const onError = vi.fn()
  useAgentWebSocket(() => {}, onError).connect()
  sockets[0].open()

  const event = new Event('error')
  sockets[0].onerror?.(event)

  expect(onError).toHaveBeenCalledWith(event)
})

test('omitting the error callback leaves socket errors harmless', () => {
  useAgentWebSocket(() => {}).connect()
  sockets[0].open()

  expect(() => sockets[0].onerror?.(new Event('error'))).not.toThrow()
})

test('exposes the underlying send/isConnected surface', () => {
  const agent = useAgentWebSocket(() => {})

  expect(agent.send('too early')).toBe(false)

  agent.connect()
  sockets[0].open()

  expect(agent.isConnected).toBe(true)
  expect(agent.send('{"message":"hi"}')).toBe(true)
  expect(sockets[0].sent).toEqual(['{"message":"hi"}'])

  agent.disconnect()
  expect(agent.socket).toBe(null)
  expect(agent.isConnected).toBe(false)
})
