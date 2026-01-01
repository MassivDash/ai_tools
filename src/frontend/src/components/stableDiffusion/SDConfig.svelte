<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import IconButton from '../ui/IconButton.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'

  import StandardSettings from './settings/StandardSettings.svelte'
  import AdvancedModels from './settings/AdvancedModels.svelte'
  import AdvancedGeneration from './settings/AdvancedGeneration.svelte'
  import SystemSettings from './settings/SystemSettings.svelte'

  import {
    SDConfigSchema,
    type SDConfig
  } from '../../validation/stableDiffusion'

  export let isOpen = false
  export let onClose = () => {}

  const dispatch = createEventDispatcher()

  let config: SDConfig = {
    output_path: './public',
    verbose: true,
    color: true,
    diffusion_model: 'z_image_turbo-Q8_0.gguf',
    llm: 'Qwen3-4B-Instruct-2507-Q8_0.gguf',
    vae: 'ae.safetensors',
    models_path: './sd_models',
    height: 1024,
    width: 1024,
    steps: null,
    batch_count: null,
    cfg_scale: 1.0,
    seed: null,
    sampling_method: null,
    scheduler: null,
    diffusion_fa: true,
    clip_l: '',
    clip_g: '',
    t5xxl: '',
    control_net: '',
    lora_model_dir: '',
    guidance: null,
    strength: null,
    // Low VRAM defaults
    offload_to_cpu: true,
    control_net_cpu: true,
    clip_on_cpu: true,
    vae_on_cpu: true,
    vae_tiling: true,
    vae_tile_size: null,
    vae_relative_tile_size: null,
    rng: 'std_default',
    threads: -1
  }

  let loading = false
  let error = ''
  let success = ''

  const loadConfig = async () => {
    loading = true
    try {
      const resp = await axiosBackendInstance.get('sd-server/config')
      if (resp.data) {
        // Backend key names match frontend, so safe to merge
        // We use spread to override defaults with loaded values
        config = { ...config, ...resp.data }

        // Ensure optional fields are handled if backend sends them
        if (resp.data.vae_tile_size === 0) config.vae_tile_size = null
        if (resp.data.vae_relative_tile_size === 0)
          config.vae_relative_tile_size = null
      }
    } catch (e) {
      console.error('Failed to load SD config', e)
    } finally {
      loading = false
    }
  }

  const saveConfig = async () => {
    loading = true
    error = ''
    success = ''
    try {
      // Validate with Zod
      const parseResult = SDConfigSchema.safeParse(config)

      if (!parseResult.success) {
        error = parseResult.error.issues
          .map((i) => `${i.path.join('.')}: ${i.message}`)
          .join(', ')
        loading = false
        return
      }

      // Sanitize: convert empty strings to null for optional API fields
      // We can use the parsed data from Zod if we set up transformers, but safe manual check is fine too
      const sanitizedConfig = { ...config }
      for (const key in sanitizedConfig) {
        const k = key as keyof SDConfig
        if (
          typeof sanitizedConfig[k] === 'string' &&
          (sanitizedConfig[k] as string).trim() === ''
        ) {
          // @ts-ignore
          sanitizedConfig[k] = null
        }
      }

      const response = await axiosBackendInstance.post(
        'sd-server/config',
        sanitizedConfig
      )

      if (response.data.success) {
        success = 'Configuration saved'
        setTimeout(() => {
          onClose()
          success = ''
        }, 1500)
      } else {
        error = response.data.message
      }
    } catch (err: any) {
      console.error('Failed to save config:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to save config'
    } finally {
      loading = false
    }
  }

  $: if (isOpen) {
    loadConfig()
  }
</script>

<div class="config-panel" class:visible={isOpen}>
  <div class="config-header">
    <div style="display: flex; align-items: center; gap: 0.75rem;">
      <MaterialIcon name="image-filter-hdr" width="28" height="28" />
      <h4>Stable Diffusion Config</h4>
    </div>
    <IconButton variant="ghost" onclick={onClose} title="Close" iconSize={24}>
      <MaterialIcon name="close" width="24" height="24" />
    </IconButton>
  </div>

  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}
    {#if success}
      <div class="success">{success}</div>
    {/if}

    <StandardSettings bind:config />
    <AdvancedModels bind:config />
    <AdvancedGeneration bind:config />
    <SystemSettings bind:config />
  </div>

  <div class="config-footer">
    <Button variant="secondary" onclick={onClose}>Cancel</Button>
    <Button variant="primary" onclick={saveConfig} disabled={loading}>
      {loading ? 'Saving...' : 'Save Configuration'}
    </Button>
  </div>
</div>

<style>
  .config-panel {
    width: 60%;
    height: 100%;
    background-color: var(--bg-primary, #fff);
    border-radius: 8px;
    border-left: 1px solid var(--border-color, #ddd);
    transform: translateX(100%);
    transition:
      transform 0.3s ease-in-out,
      background-color 0.3s ease,
      border-color 0.3s ease;
    z-index: 10;
    display: flex;
    flex-direction: column;
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    box-shadow: -2px 0 8px var(--shadow, rgba(0, 0, 0, 0.1));
    max-height: 100vh;
  }

  .config-panel.visible {
    transform: translateX(0);
  }

  .config-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-top-left-radius: 8px;
    border-bottom-left-radius: 8px;
    border-bottom: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease;
  }

  .config-header h4 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    font-size: 1.2rem;
    font-weight: 600;
  }

  .config-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .config-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease;
  }

  .error {
    color: var(--accent-color);
    background: rgba(255, 100, 100, 0.1);
    padding: 0.5rem;
    border-radius: 4px;
    font-size: 0.9rem;
  }

  .success {
    color: var(--success-color, #28a745);
    background: rgba(40, 167, 69, 0.1);
    padding: 0.5rem;
    border-radius: 4px;
    font-size: 0.9rem;
  }

  @media screen and (max-width: 768px) {
    .config-panel {
      width: 100%;
    }
  }
</style>
