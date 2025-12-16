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
  let _loading = false
  let _error = ''
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
    return wsUrl
  }

  const connectWebSocket = () => {
    try {
      const wsUrl = getWebSocketUrl()
      ws = new WebSocket(wsUrl)

      ws.onopen = () => {
        isConnected = true
        _error = ''
        _loading = false
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
        _error = 'WebSocket connection error'
      }

      ws.onclose = (_event) => {
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
      _error = err.message || 'Failed to connect to logs stream'
    }
  }

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp * 1000)
    return date.toLocaleTimeString()
  }

  onMount(() => {
    _loading = true
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
    background-color: var(--md-surface);
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--md-outline-variant);
    box-shadow: 0 2px 8px var(--md-shadow, rgba(0, 0, 0, 0.1));
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      box-shadow 0.3s ease;
  }

  .terminal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background-color: var(--md-surface-variant);
    border-bottom: 1px solid var(--md-outline-variant);
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .terminal-header h4 {
    margin: 0;
    color: var(--md-on-surface);
    font-size: 0.9375rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    transition: color 0.3s ease;
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
    padding: 0.75rem 1rem;
    font-family:
      'SF Mono',
      'Monaco',
      'Inconsolata',
      'Roboto Mono',
      'Source Code Pro',
      Menlo,
      Consolas,
      'DejaVu Sans Mono',
      monospace;
    font-size: 0.875rem;
    line-height: 1.6;
    background-color: var(--md-surface);
    color: var(--md-on-surface);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  /* Dark theme: make terminal background darker for better terminal feel */
  html.dark .terminal-content {
    background-color: color-mix(in srgb, var(--md-surface) 95%, black);
  }

  /* Light theme: slightly off-white for better readability */
  html:not(.dark) .terminal-content {
    background-color: color-mix(in srgb, var(--md-surface) 98%, white);
  }

  .log-line {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 0.35rem;
    word-wrap: break-word;
    align-items: flex-start;
  }

  .log-line.stdout {
    color: var(--md-on-surface);
    transition: color 0.3s ease;
  }

  .log-line.stderr {
    color: var(--md-tertiary);
    transition: color 0.3s ease;
  }

  /* Dark theme: use a softer warning color for stderr */
  html.dark .log-line.stderr {
    color: #ffab40; /* Amber/orange warning color for dark theme */
  }

  /* Light theme: use a muted warning color for stderr */
  html:not(.dark) .log-line.stderr {
    color: #d97706; /* Muted orange/amber for light theme */
  }

  .log-timestamp {
    color: var(--md-on-surface-variant);
    flex-shrink: 0;
    min-width: 85px;
    font-size: 0.8125rem;
    opacity: 0.8;
    transition: color 0.3s ease;
  }

  .log-source {
    color: var(--md-on-surface-variant);
    flex-shrink: 0;
    min-width: 75px;
    font-weight: 500;
    font-size: 0.8125rem;
    opacity: 0.75;
    transition: color 0.3s ease;
  }

  .log-text {
    flex: 1;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.875rem;
  }

  .empty-logs {
    color: var(--md-on-surface-variant);
    text-align: center;
    padding: 2rem;
    font-style: italic;
    opacity: 0.7;
    transition: color 0.3s ease;
  }

  /* Custom scrollbar styling - Webkit browsers (Chrome, Safari, Edge) */
  .terminal-content::-webkit-scrollbar {
    width: 10px;
  }

  .terminal-content::-webkit-scrollbar-track {
    background: var(--md-surface-variant, rgba(0, 0, 0, 0.05));
    border-radius: 10px;
    margin: 4px 0;
    transition: background-color 0.3s ease;
  }

  .terminal-content::-webkit-scrollbar-thumb {
    background: var(--md-outline, rgba(0, 0, 0, 0.3));
    border-radius: 10px;
    border: 2px solid var(--md-surface);
    transition: background-color 0.2s ease;
  }

  .terminal-content::-webkit-scrollbar-thumb:hover {
    background: var(--md-outline-variant, rgba(0, 0, 0, 0.5));
  }

  /* Custom scrollbar styling - Firefox */
  .terminal-content {
    scrollbar-width: thin;
    scrollbar-color: var(--md-outline, rgba(0, 0, 0, 0.3))
      var(--md-surface-variant, rgba(0, 0, 0, 0.05));
  }

  /* Dark theme scrollbar adjustments */
  @media (prefers-color-scheme: dark) {
    .terminal-content::-webkit-scrollbar-track {
      background: var(--md-surface-variant, rgba(255, 255, 255, 0.05));
    }

    .terminal-content::-webkit-scrollbar-thumb {
      background: var(--md-outline, rgba(255, 255, 255, 0.3));
    }

    .terminal-content::-webkit-scrollbar-thumb:hover {
      background: var(--md-outline-variant, rgba(255, 255, 255, 0.5));
    }

    .terminal-content {
      scrollbar-color: var(--md-outline, rgba(255, 255, 255, 0.3))
        var(--md-surface-variant, rgba(255, 255, 255, 0.05));
    }
  }
</style>
