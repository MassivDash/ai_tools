<script lang="ts">
  import { onMount, onDestroy } from 'svelte'

  interface LogLine {
    timestamp: number
    line: string
    source: 'stdout' | 'stderr'
  }

  type WebSocketMessage =
    | { type: 'log'; log: LogLine }
    | { type: 'logs_batch'; logs: LogLine[] }

  let logs: LogLine[] = []
  let loading = false
  let error = ''
  let ws: WebSocket | null = null
  let isConnected = false
  let terminalRef: HTMLDivElement
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null

  const getWebSocketUrl = (): string => {
    let baseUrl = import.meta.env.PUBLIC_API_URL || window.location.origin
    // Remove /api suffix if present (PUBLIC_API_URL includes /api)
    baseUrl = baseUrl.replace(/\/api\/?$/, '')
    // Remove trailing slash
    baseUrl = baseUrl.replace(/\/$/, '')
    const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws'
    const wsBase = baseUrl.replace(/^https?:\/\//, '')
    const wsUrl = `${wsProtocol}://${wsBase}/api/llama-server/logs/ws`
    console.log(
      '🔗 Constructed WebSocket URL:',
      wsUrl,
      'from base:',
      import.meta.env.PUBLIC_API_URL
    )
    return wsUrl
  }

  const connectWebSocket = () => {
    try {
      const wsUrl = getWebSocketUrl()
      console.log('🔌 Connecting to logs WebSocket:', wsUrl)
      ws = new WebSocket(wsUrl)

      ws.onopen = () => {
        console.log('✅ Logs WebSocket connected')
        isConnected = true
        error = ''
        loading = false
      }

      ws.onmessage = (event) => {
        try {
          const message: WebSocketMessage = JSON.parse(event.data)

          if (message.type === 'log') {
            logs = [...logs, message.log]
            // Auto-scroll to bottom
            if (terminalRef) {
              setTimeout(() => {
                terminalRef.scrollTop = terminalRef.scrollHeight
              }, 10)
            }
          } else if (message.type === 'logs_batch') {
            logs = message.logs
            // Auto-scroll to bottom
            if (terminalRef) {
              setTimeout(() => {
                terminalRef.scrollTop = terminalRef.scrollHeight
              }, 10)
            }
          }
        } catch (err) {
          console.error('❌ Failed to parse WebSocket message:', err)
        }
      }

      ws.onerror = (err) => {
        console.error('❌ Logs WebSocket error:', err)
        isConnected = false
        error = 'WebSocket connection error'
      }

      ws.onclose = (event) => {
        console.log('🔌 Logs WebSocket closed, reconnecting...', {
          code: event.code,
          reason: event.reason,
          wasClean: event.wasClean
        })
        isConnected = false
        ws = null
        // Reconnect after 2 seconds
        if (reconnectTimeout) {
          clearTimeout(reconnectTimeout)
        }
        reconnectTimeout = setTimeout(() => {
          connectWebSocket()
        }, 2000)
      }
    } catch (err: any) {
      console.error('❌ Failed to connect WebSocket:', err)
      error = err.message || 'Failed to connect to logs stream'
    }
  }

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp * 1000)
    return date.toLocaleTimeString()
  }

  onMount(() => {
    loading = true
    connectWebSocket()
  })

  onDestroy(() => {
    if (ws) {
      ws.close()
      ws = null
    }
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
    }
  })
</script>

<div class="terminal-container">
  <div class="terminal-header">
    <h4>Server Output</h4>
    <div class="header-status">
      {#if isConnected}
        <span class="status-indicator connected" title="Connected">🟢</span>
      {:else}
        <span class="status-indicator disconnected" title="Disconnected"
          >🔴</span
        >
      {/if}
    </div>
  </div>
  <div class="terminal-content" bind:this={terminalRef}>
    {#if logs.length === 0}
      <div class="empty-logs">No logs yet. Start the server to see output.</div>
    {:else}
      {#each logs as logEntry}
        <div class="log-line {logEntry.source}">
          <span class="log-timestamp"
            >{formatTimestamp(logEntry.timestamp)}</span
          >
          <span class="log-source">[{logEntry.source}]</span>
          <span class="log-text">{logEntry.line}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .terminal-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: #1e1e1e;
    border-radius: 4px;
    overflow: hidden;
  }

  .terminal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 1rem;
    background-color: #2d2d2d;
    border-bottom: 1px solid #444;
  }

  .terminal-header h4 {
    margin: 0;
    color: #fff;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .header-status {
    display: flex;
    align-items: center;
  }

  .status-indicator {
    font-size: 0.75rem;
    margin-right: 0.5rem;
  }

  .status-indicator.connected {
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }

  .terminal-content {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    font-family: 'Courier New', monospace;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .log-line {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
    word-wrap: break-word;
  }

  .log-line.stdout {
    color: #d4d4d4;
  }

  .log-line.stderr {
    color: #f48771;
  }

  .log-timestamp {
    color: #858585;
    flex-shrink: 0;
    min-width: 80px;
  }

  .log-source {
    color: #858585;
    flex-shrink: 0;
    min-width: 70px;
    font-weight: 600;
  }

  .log-text {
    flex: 1;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .empty-logs {
    color: #858585;
    text-align: center;
    padding: 2rem;
    font-style: italic;
  }

  /* Scrollbar styling */
  .terminal-content::-webkit-scrollbar {
    width: 8px;
  }

  .terminal-content::-webkit-scrollbar-track {
    background: #1e1e1e;
  }

  .terminal-content::-webkit-scrollbar-thumb {
    background: #444;
    border-radius: 4px;
  }

  .terminal-content::-webkit-scrollbar-thumb:hover {
    background: #555;
  }
</style>
