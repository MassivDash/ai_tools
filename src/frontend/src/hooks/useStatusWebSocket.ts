import { useWebSocket, type WebSocketOptions } from './useWebSocket'

export interface LlamaServerStatus {
  active: boolean
  port: number
}

export interface StatusWebSocketMessage {
  type: 'status'
  active: boolean
  port: number
}

export function useStatusWebSocket(
  onStatusChange: (_status: LlamaServerStatus) => void,
  onServerReady?: () => void
) {
  const getWebSocketUrl = (): string => {
    let baseUrl = import.meta.env.PUBLIC_API_URL || window.location.origin
    baseUrl = baseUrl.replace(/\/api\/?$/, '')
    baseUrl = baseUrl.replace(/\/$/, '')
    const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws'
    const wsBase = baseUrl.replace(/^https?:\/\//, '')
    return `${wsProtocol}://${wsBase}/api/llama-server/status/ws`
  }

  let previousStatus: LlamaServerStatus | null = null

  const options: WebSocketOptions = {
    url: getWebSocketUrl(),
    onMessage: (event) => {
      try {
        const message: StatusWebSocketMessage = JSON.parse(event.data)
        if (message.type === 'status') {
          const newStatus: LlamaServerStatus = {
            active: message.active,
            port: message.port
          }

          // Only call onStatusChange if status actually changed
          if (
            !previousStatus ||
            previousStatus.active !== newStatus.active ||
            previousStatus.port !== newStatus.port
          ) {
            onStatusChange(newStatus)

            // Call onServerReady if server just became active
            if (
              newStatus.active &&
              previousStatus &&
              !previousStatus.active &&
              onServerReady
            ) {
              onServerReady()
            }
          }

          previousStatus = newStatus
        }
      } catch (err) {
        console.error('❌ Failed to parse status WebSocket message:', err)
      }
    }
  }

  return useWebSocket(options)
}
