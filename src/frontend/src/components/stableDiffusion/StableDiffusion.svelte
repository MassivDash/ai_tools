<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import Button from '../ui/Button.svelte'
  import Input from '../ui/Input.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import SDConfig from './SDConfig.svelte'
  import Terminal from './Terminal.svelte'
  import Gallery from './Gallery.svelte'
  import { GenerationSchema } from '../../validation/stableDiffusion'

  import { slide } from 'svelte/transition'

  let prompt = ''
  let negative_prompt = ''
  let isGenerating = false
  let error = ''
  let showConfig = false
  let showTerminal = false
  let galleryComponent: Gallery

  let currentModel = 'Loading...'
  let currentWidth: number | null = null
  let currentHeight: number | null = null
  let currentOffload = false
  let showNegative = false

  let ws: WebSocket | null = null
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null

  const getWebSocketUrl = (): string => {
    let baseUrl = import.meta.env.PUBLIC_API_URL || window.location.origin
    baseUrl = baseUrl.replace(/\/api\/?$/, '').replace(/\/$/, '')
    const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws'
    const wsBase = baseUrl.replace(/^https?:\/\//, '')
    return `${wsProtocol}://${wsBase}/api/sd-server/logs/ws`
  }

  const connectWebSocket = () => {
    try {
      ws = new WebSocket(getWebSocketUrl())
      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data)
          if (msg.type === 'status') {
            const wasGenerating = isGenerating
            isGenerating = msg.is_generating

            if ((wasGenerating && !isGenerating) || msg.current_file) {
              if (galleryComponent) galleryComponent.refresh()
              if (wasGenerating && !isGenerating) {
                showTerminal = false
              }
            }
          } else if (msg.type === 'error') {
            error = msg.message
            isGenerating = false
          }
        } catch (e) {
          console.error('Failed to parse SD status', e)
        }
      }
      ws.onclose = () => {
        reconnectTimeout = setTimeout(connectWebSocket, 2000)
      }
    } catch (e) {
      console.error('Failed to connect SD WS', e)
    }
  }

  const fetchCurrentConfig = async () => {
    try {
      const resp = await axiosBackendInstance.get('sd-server/config')
      if (resp.data) {
        if (resp.data.diffusion_model) currentModel = resp.data.diffusion_model
        if (resp.data.width) currentWidth = resp.data.width
        if (resp.data.height) currentHeight = resp.data.height
        if (resp.data.offload_to_cpu !== undefined)
          currentOffload = resp.data.offload_to_cpu
      }
    } catch (e) {
      console.error('Failed to fetch config for badge', e)
    }
  }

  onMount(() => {
    connectWebSocket()
    fetchCurrentConfig()
  })

  onDestroy(() => {
    if (ws) ws.close()
    if (reconnectTimeout) clearTimeout(reconnectTimeout)
  })

  const generateImage = async () => {
    error = ''

    // Validate with Zod
    const validation = GenerationSchema.safeParse({ prompt, negative_prompt })
    if (!validation.success) {
      error = validation.error.issues[0].message
      return
    }

    try {
      await axiosBackendInstance.post('sd-server/config', {
        prompt,
        negative_prompt
      })

      const response = await axiosBackendInstance.post('sd-server/start')
      if (response.data.success) {
        showTerminal = true
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('Failed to start generation:', err)
      error =
        err.response?.data?.message ||
        err.message ||
        'Failed to start generation'
    }
  }

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      if (!isGenerating) generateImage()
    }
  }

  const handleConfigClose = () => {
    showConfig = false
    fetchCurrentConfig()
  }
</script>

<div class="sd-page">
  <PageSubHeader title="Stable Diffusion" icon="image">
    {#snippet actions()}
      <Button
        variant="info"
        class="button-icon-only"
        onclick={() => (showConfig = !showConfig)}
        title="Configuration"
      >
        <MaterialIcon name="cog" width="32" height="32" />
      </Button>
      <Button
        variant="info"
        class="button-icon-only"
        onclick={() => (showTerminal = !showTerminal)}
        title={showTerminal ? 'Hide Terminal' : 'Show Terminal'}
      >
        <MaterialIcon name="console" width="32" height="32" />
      </Button>
    {/snippet}
  </PageSubHeader>

  <div
    class="content-area"
    class:has-terminal={showTerminal}
    class:has-config={showConfig}
  >
    <div class="terminal-sidebar" class:visible={showTerminal}>
      <Terminal />
    </div>

    <div
      class="main-content"
      class:with-terminal={showTerminal}
      class:with-config={showConfig}
    >
      <div class="controls-section">
        <div class="input-header">
          <div class="badges">
            <span class="model-badge">
              <MaterialIcon name="cube-scan" width="16" height="16" />
              {currentModel}
            </span>
            {#if currentWidth && currentHeight}
              <span class="size-badge">
                {currentWidth}x{currentHeight}
              </span>
            {/if}
            {#if currentOffload}
              <span class="cpu-badge" title="Offload to CPU enabled">
                CPU
              </span>
            {/if}
          </div>

          <Button
            variant="ghost"
            size="small"
            onclick={() => (showNegative = !showNegative)}
            title="Toggle Negative Prompt"
          >
            <MaterialIcon
              name={showNegative ? 'eye-off' : 'eye'}
              width="16"
              height="16"
            />
            Negative Prompt
          </Button>
        </div>

        <div class="prompt-container">
          <div class="main-input">
            <Input
              label=""
              bind:value={prompt}
              placeholder="Describe your image..."
              multiline={true}
              rows={2}
              onkeydown={handleKeydown}
            />
          </div>
          <Button
            variant="primary"
            onclick={generateImage}
            disabled={isGenerating}
            class="generate-btn"
            title="Generate Image (Enter)"
          >
            {#if isGenerating}
              <div class="spinning">
                <MaterialIcon name="loading" width="24" height="24" />
              </div>
            {:else}
              <MaterialIcon name="creation" width="24" height="24" />
            {/if}
          </Button>
        </div>

        {#if showNegative}
          <div class="negative-input" transition:slide>
            <Input
              label="Negative Prompt"
              bind:value={negative_prompt}
              placeholder="What to avoid..."
              onkeydown={(e) => {
                if (e.key === 'Enter') generateImage()
              }}
            />
          </div>
        {/if}

        {#if error}
          <div class="error-banner">{error}</div>
        {/if}
      </div>

      <div class="gallery-section">
        <Gallery bind:this={galleryComponent} {isGenerating} />
      </div>
    </div>

    <SDConfig isOpen={showConfig} onClose={handleConfigClose} />
  </div>
</div>

<style>
  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
  .spinning {
    animation: spin 1s linear infinite;
    display: flex;
  }

  .sd-page {
    width: 100%;
    min-height: 80vh;
    display: flex;
    flex-direction: column;
    background-color: var(--bg-primary);
    overflow: hidden;
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: row;
    position: relative;
    overflow: hidden;
    height: calc(100vh - 120px);
  }

  .terminal-sidebar {
    width: 40%;
    min-width: 400px;
    height: 100%;
    background-color: #1e1e1e;
    position: absolute;
    left: 0;
    top: 0;
    transform: translateX(-100%);
    transition: transform 0.3s ease-in-out;
    z-index: 10;
    border-right: 1px solid var(--border-color);
  }

  .terminal-sidebar.visible {
    transform: translateX(0);
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1rem;
    overflow-y: auto;
    transition: margin 0.3s ease-in-out;
    width: 100%;
  }

  .main-content.with-terminal {
    margin-left: max(40%, 400px);
  }

  .main-content.with-config {
    margin-right: 400px;
  }

  .controls-section {
    background-color: var(--md-surface);
    padding: 1rem;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
    margin-bottom: 2rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 0.5rem;
  }

  .badges {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .model-badge {
    font-size: 0.75rem;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    padding: 0.2rem 0.6rem;
    border-radius: 12px;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-weight: 600;
    opacity: 0.8;
  }

  .size-badge {
    font-size: 0.7rem;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    padding: 0.2rem 0.5rem;
    border-radius: 8px;
    font-family: monospace;
    opacity: 0.8;
  }

  .cpu-badge {
    font-size: 0.7rem;
    background: var(--info-color-alpha-10, rgba(0, 150, 255, 0.1));
    color: var(--info-color, #0096ff);
    border: 1px solid var(--info-color, #0096ff);
    padding: 0.1rem 0.4rem;
    border-radius: 8px;
    font-weight: 700;
  }

  .prompt-container {
    display: flex;
    gap: 0.5rem;
    align-items: stretch;
  }

  .main-input {
    flex: 1;
  }

  /* Targeting the generate button specifically for layout */
  :global(.generate-btn) {
    height: 48px;
    min-width: 48px; /* Smaller width */
    padding: 0 0.75rem; /* Reduce padding */
    border-radius: 8px;
  }

  .negative-input {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-secondary); /* Slight contrast */
    border-radius: 8px;
  }

  .error-banner {
    background-color: var(--md-error-container);
    color: var(--md-error);
    padding: 0.75rem;
    border-radius: 8px;
    font-size: 0.9rem;
    margin-top: 0.5rem;
  }

  .gallery-section {
    flex: 1;
    min-height: 0;
  }

  @media (max-width: 768px) {
    .main-content.with-terminal {
      margin-left: 0;
    }
    .terminal-sidebar {
      width: 100%;
      min-width: 100%;
    }
  }
</style>
