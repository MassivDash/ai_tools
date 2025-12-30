export interface WebSocketOptions {
  url: string
  onOpen?: () => void
  onMessage?: (_event: MessageEvent) => void
  onError?: (_event: Event) => void
  onClose?: (_event: CloseEvent) => void
  reconnectInterval?: number
  autoReconnect?: boolean
}

export function useWebSocket(options: WebSocketOptions) {
  let ws: WebSocket | null = null
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null
  let isConnected = false

  const connect = () => {
    try {
      ws = new WebSocket(options.url)

      ws.onopen = () => {
        isConnected = true
        options.onOpen?.()
      }

      ws.onmessage = (event) => {
        options.onMessage?.(event)
      }

      ws.onerror = (event) => {
        console.error('WebSocket error:', event)
        isConnected = false
        options.onError?.(event)
      }

      ws.onclose = (event) => {
        isConnected = false
        options.onClose?.(event)

        if (options.autoReconnect !== false) {
          if (reconnectTimeout) {
            clearTimeout(reconnectTimeout)
          }
          reconnectTimeout = setTimeout(() => {
            connect()
          }, options.reconnectInterval || 2000)
        }
      }
    } catch (err) {
      console.error('Failed to connect WebSocket:', err)
      options.onError?.(err as Event)
    }
  }

  const disconnect = () => {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
      reconnectTimeout = null
    }
    if (ws) {
      ws.close()
      ws = null
    }
    isConnected = false
  }

  const send = (data: string | ArrayBuffer | Blob) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(data)
      return true
    }
    return false
  }

  return {
    connect,
    disconnect,
    send,
    get isConnected() {
      return isConnected && ws?.readyState === WebSocket.OPEN
    },
    get socket() {
      return ws
    }
  }
}
