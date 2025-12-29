import { useWebSocket } from './useWebSocket'
import type { WebSocketOptions } from './useWebSocket'

export interface AgentStreamEvent {
  type: 'status' | 'tool_call' | 'tool_result' | 'text_chunk' | 'done' | 'error'
  status?: string
  message?: string
  tool_name?: string
  arguments?: string
  success?: boolean
  result?: string
  text?: string
  conversation_id?: string
  tool_calls?: Array<{
    tool_name: string
    result: string
  }>
}

export function useAgentWebSocket(
  onEvent: (_event: AgentStreamEvent) => void,
  _onError?: (_error: Event) => void
) {
  const getWebSocketUrl = (): string => {
    let baseUrl = import.meta.env.PUBLIC_API_URL || window.location.origin
    baseUrl = baseUrl.replace(/\/api\/?$/, '')
    baseUrl = baseUrl.replace(/\/$/, '')
    const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws'
    const wsBase = baseUrl.replace(/^https?:\/\//, '')
    return `${wsProtocol}://${wsBase}/api/agent/stream/ws`
  }

  const options: WebSocketOptions = {
    url: getWebSocketUrl(),
    onMessage: (event) => {
      try {
        // Event is sent directly as AgentStreamEvent (no wrapper)
        const streamEvent: AgentStreamEvent = JSON.parse(event.data)
        onEvent(streamEvent)
      } catch (err) {
        console.error('❌ Failed to parse agent WebSocket message:', err)
      }
    },
    onError: _onError,
    autoReconnect: true,
    reconnectInterval: 2000
  }

  return useWebSocket(options)
}
