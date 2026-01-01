<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import SDConfig from './SDConfig.svelte'
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import Input from '../ui/Input.svelte'

  let loading = false
  let error = ''
  let message = ''
  let showConfig = false
  let showTerminal = false

  let prompt = 'Spiderman swinging through new york, comic 50s style'
  let generatedImage = ''

  const startGeneration = async () => {
    loading = true
    error = ''
    message = ''
    try {
      // First update the prompt in the config (optional, or pass it directly if the endpoint supports overriding)
      // My endpoint reads from config, but I should probably allow passing prompt in start body or update config first.
      // For now, let's update config with the prompt before starting.
      await axiosBackendInstance.post('sd-server/config', { prompt })

      const response = await axiosBackendInstance.post<any>('sd-server/start')
      if (response.data.success) {
        message = response.data.message
        // Extract filename from message or similar?
        // The endpoint returns: "SD generation started. Output: image_TIMESTAMP.png"
        // In a real app we'd want a better way to get the path.
        // For now, let's just show the message.
        if (message.includes('Output: ')) {
          const filename = message.split('Output: ')[1]
          // Assuming we serve images statically or something?
          // We haven't set up a static file server for the images folder.
          // But we can just show the message for now.
        }
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('Failed to start generation:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to start generation'
    } finally {
      loading = false
    }
  }
</script>

<div class="sd-container">
  <PageSubHeader title="Stable Diffusion" icon="image-filter-hdr">
    {#snippet actions()}
      <Button
        variant="info"
        class="button-icon-only"
        onclick={() => (showConfig = !showConfig)}
        title="Config"
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

  <div class="content-area">
    <div class="main-content" class:with-terminal={showTerminal}>
      <div class="control-panel">
        <div class="prompt-input">
          <Input
            label="Prompt"
            bind:value={prompt}
            placeholder="Enter your prompt here..."
          />
        </div>
        <div class="generate-btn">
          <Button
            variant="success"
            onclick={startGeneration}
            disabled={loading}
            title={loading ? 'Generating...' : 'Generate Image'}
          >
            <MaterialIcon name="play" width="24" height="24" />
            Generate
          </Button>
        </div>
      </div>

      {#if error}
        <div class="error-banner">{error}</div>
      {/if}
      {#if message}
        <div class="success-banner">{message}</div>
      {/if}

      <div class="preview-area">
        {#if generatedImage}
          <img src={generatedImage} alt="Generated Preview" />
        {:else}
          <div class="placeholder">
            <MaterialIcon name="image-outline" width="64" height="64" />
            <p>Generated image will appear here</p>
          </div>
        {/if}
      </div>
    </div>

    {#if showTerminal}
      <div class="terminal-sidebar">
        <div class="terminal-network">
          <div class="terminal-header">
            <h4>Stable Diffusion Logs</h4>
          </div>
          <div class="terminal-body">
            <p class="log-info">Real-time logs not implemented yet.</p>
            {#if message}
              <p class="log-success">> {message}</p>
            {/if}
            {#if error}
              <p class="log-error">> Error: {error}</p>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </div>

  <SDConfig isOpen={showConfig} onClose={() => (showConfig = false)} />
</div>

<style>
  .sd-container {
    width: 100%;
    display: flex;
    flex-direction: column;
    min-height: 80vh;
  }

  .content-area {
    display: flex;
    flex: 1;
    position: relative;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1rem;
    gap: 1rem;
    transition: margin-right 0.3s ease;
  }

  .main-content.with-terminal {
    margin-right: 400px;
  }

  .control-panel {
    display: flex;
    gap: 1rem;
    align-items: flex-end;
  }

  .prompt-input {
    flex: 1;
  }

  .preview-area {
    flex: 1;
    background: var(--md-surface-variant);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 400px;
  }

  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    color: var(--md-on-surface-variant);
    opacity: 0.5;
  }

  .terminal-sidebar {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 400px;
    background: #1e1e1e;
    color: #fff;
    border-left: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
  }

  .terminal-header {
    padding: 0.5rem 1rem;
    border-bottom: 1px solid #333;
    background: #252526;
  }

  .terminal-body {
    padding: 1rem;
    font-family: monospace;
    font-size: 0.9rem;
  }

  .log-info {
    color: #aaa;
  }
  .log-success {
    color: #4caf50;
  }
  .log-error {
    color: #f44336;
  }

  .error-banner {
    background: rgba(255, 0, 0, 0.1);
    color: #d32f2f;
    padding: 1rem;
    border-radius: 4px;
  }

  .success-banner {
    background: rgba(0, 255, 0, 0.1);
    color: #388e3c;
    padding: 1rem;
    border-radius: 4px;
  }
</style>
