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

  let prompt = ''
  let negative_prompt = ''
  let isGenerating = false
  let error = ''
  let showConfig = false
  let showTerminal = false
  let galleryComponent: Gallery

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
            isGenerating = false // Ensure we stop spinning
            // Keep terminal open on error so user can see logs?
            // User complained it 'closed like everything ended', so maybe we keep it open.
            // But we also show the error banner.
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

  onMount(() => {
    connectWebSocket()
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
        <div class="input-group">
          <Input
            label="Prompt"
            bind:value={prompt}
            placeholder="Describe your image..."
            multiline={true}
            rows={3}
            onkeydown={handleKeydown}
          />
        </div>
        <div class="input-group">
          <Input
            label="Negative Prompt"
            bind:value={negative_prompt}
            placeholder="What to avoid..."
            onkeydown={(e) => {
              if (e.key === 'Enter') generateImage()
            }}
          />
        </div>

        {#if error}
          <div class="error-banner">{error}</div>
        {/if}

        <div class="action-bar">
          <Button
            variant="primary"
            onclick={generateImage}
            disabled={isGenerating}
            isLoading={isGenerating}
          >
            {isGenerating ? 'Generating...' : 'Generate Image'}
            <MaterialIcon name="creation" width="20" height="20" slot="icon" />
          </Button>
        </div>
      </div>

      <div class="gallery-section">
        <Gallery bind:this={galleryComponent} {isGenerating} />
      </div>
    </div>

    <SDConfig isOpen={showConfig} onClose={() => (showConfig = false)} />
  </div>
</div>

<style>
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
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
    margin-bottom: 2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .input-group {
    width: 100%;
  }

  .action-bar {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.5rem;
  }

  .error-banner {
    background-color: var(--md-error-container);
    color: var(--md-error);
    padding: 0.75rem;
    border-radius: 8px;
    font-size: 0.9rem;
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
