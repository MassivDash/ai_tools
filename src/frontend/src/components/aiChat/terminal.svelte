<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { onMount, onDestroy } from 'svelte'

  interface LogLine {
    timestamp: number
    line: string
    source: 'stdout' | 'stderr'
  }

  interface LogsResponse {
    logs: LogLine[]
  }

  let logs: LogLine[] = []
  let loading = false
  let error = ''
  let logInterval: ReturnType<typeof setInterval> | null = null
  let terminalRef: HTMLDivElement

  const loadLogs = async () => {
    try {
      const response = await axiosBackendInstance.get<LogsResponse>(
        'llama-server/logs'
      )
      logs = response.data.logs
      // Auto-scroll to bottom
      if (terminalRef) {
        setTimeout(() => {
          terminalRef.scrollTop = terminalRef.scrollHeight
        }, 10)
      }
    } catch (err: any) {
      console.error('❌ Failed to load logs:', err)
      error = err.response?.data?.error || err.message || 'Failed to load logs'
    }
  }

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp * 1000)
    return date.toLocaleTimeString()
  }

  onMount(() => {
    loadLogs()
    // Poll for logs every 500ms
    logInterval = setInterval(() => {
      loadLogs()
    }, 500)
  })

  onDestroy(() => {
    if (logInterval) {
      clearInterval(logInterval)
    }
  })
</script>

<div class="terminal-container">
  <div class="terminal-header">
    <h4>Server Output</h4>
    <button class="refresh-button" onclick={loadLogs} title="Refresh logs">
      🔄
    </button>
  </div>
  <div class="terminal-content" bind:this={terminalRef}>
    {#if logs.length === 0}
      <div class="empty-logs">No logs yet. Start the server to see output.</div>
    {:else}
      {#each logs as logEntry}
        <div class="log-line {logEntry.source}">
          <span class="log-timestamp">{formatTimestamp(logEntry.timestamp)}</span>
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

  .refresh-button {
    background: none;
    border: none;
    color: #fff;
    cursor: pointer;
    font-size: 1rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    transition: background-color 0.2s;
  }

  .refresh-button:hover {
    background-color: #444;
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

