<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import IconButton from '../ui/IconButton.svelte'
  import Accordion from '../ui/Accordion.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import Input from '../ui/Input.svelte'
  import CheckboxWithHelp from '../ui/CheckboxWithHelp.svelte'
  import LabelWithHelp from '../ui/LabelWithHelp.svelte'

  export let isOpen = false
  export let onClose = () => {}

  const dispatch = createEventDispatcher()

  let config: any = {
    // Defaults matching backend types.rs
    output_path: './public',
    verbose: true,
    color: true,
    mode: null,
    diffusion_model: 'z_image_turbo-Q8_0.gguf',
    llm: 'Qwen3-4B-Instruct-2507-Q8_0.gguf',
    vae: 'ae.safetensors',
    models_path: './sd_models',
    prompt: '', // Usually overridden by main UI
    height: 1024,
    width: 1024,
    steps: null,
    batch_count: null,
    cfg_scale: 1.0,
    seed: null,
    sampling_method: null,
    scheduler: null,
    diffusion_fa: true,

    // Optional/Advanced fields
    clip_l: '',
    clip_g: '',
    t5xxl: '',
    control_net: '',
    lora_model_dir: '',
    preview_method: '',
    guidance: null,
    strength: null,
    offload_to_cpu: false,
    rng: 'std_default',
    threads: -1
  }

  let loading = false
  let error = ''
  let success = ''

  const loadConfig = async () => {
    // TODO: Fetch existing config from backend if state persistency is needed
    // For now we rely on defaults or what was last set in this session
  }

  const saveConfig = async () => {
    loading = true
    error = ''
    success = ''
    try {
      // Clean up empty optional strings to null/undefined if necessary,
      // but backend optional fields handle string->Option conversion if we send them as fields.
      // Actually types.rs expects Option<String>.
      // Axios/JSON serialization of empty string "" vs null might matter.
      // Let's send the config object as is; the backend matching should handle it
      // IF the backend deserialize can handle "" as None.
      // Rust Serde Option deserialization usually treats missing or null as None.
      // "" is Some("").
      // To be safe, let's sanitize empty strings to null for Option fields.
      const sanitizedConfig = { ...config }
      ;[
        'clip_l',
        'clip_g',
        't5xxl',
        'control_net',
        'lora_model_dir',
        'preview_method',
        'model',
        'init_img',
        'mask',
        'control_image',
        'taesd',
        'embd_dir',
        'upscale_model'
      ].forEach((key) => {
        if (sanitizedConfig[key] === '') sanitizedConfig[key] = null
      })

      const response = await axiosBackendInstance.post(
        'sd-server/config',
        sanitizedConfig
      )
      if (response.data.success) {
        success = 'Configuration saved successfully'
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
    <IconButton
      variant="ghost"
      onclick={onClose}
      title="Close"
      aria-label="Close"
      iconSize={24}
    >
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

    <!-- Standard / Basic Settings -->
    <div class="section-title">Standard Settings</div>

    <div class="config-section">
      <LabelWithHelp
        id="diffusion-model"
        label="Diffusion Model"
        helpText="Path to the standalone diffusion model (GGUF)"
      />
      <Input
        id="diffusion-model"
        label=""
        bind:value={config.diffusion_model}
        placeholder="z_image_turbo-Q8_0.gguf"
      />
    </div>

    <div class="config-section">
      <LabelWithHelp
        id="vae"
        label="VAE"
        helpText="Path to standalone VAE model"
      />
      <Input
        id="vae"
        label=""
        bind:value={config.vae}
        placeholder="ae.safetensors"
      />
    </div>

    <div class="config-section">
      <LabelWithHelp
        id="llm"
        label="LLM / CLIP"
        helpText="Path to the LLM or text encoder"
      />
      <Input
        id="llm"
        label=""
        bind:value={config.llm}
        placeholder="Qwen3-4B-Instruct-2507-Q8_0.gguf"
      />
    </div>

    <div class="form-row">
      <div class="form-group half">
        <LabelWithHelp id="height" label="Height" helpText="Image height" />
        <Input id="height" label="" type="number" bind:value={config.height} />
      </div>
      <div class="form-group half">
        <LabelWithHelp id="width" label="Width" helpText="Image width" />
        <Input id="width" label="" type="number" bind:value={config.width} />
      </div>
    </div>

    <div class="form-row">
      <div class="form-group half">
        <LabelWithHelp id="steps" label="Steps" helpText="Sample steps" />
        <Input id="steps" label="" type="number" bind:value={config.steps} />
      </div>
      <div class="form-group half">
        <LabelWithHelp
          id="cfg-scale"
          label="CFG Scale"
          helpText="Guidance scale"
        />
        <Input
          id="cfg-scale"
          label=""
          type="number"
          step="0.1"
          bind:value={config.cfg_scale}
        />
      </div>
    </div>

    <div class="config-section">
      <CheckboxWithHelp
        bind:checked={config.diffusion_fa}
        label="Flash Attention"
        helpText="Use flash attention in diffusion model"
      />
    </div>

    <div class="config-section">
      <CheckboxWithHelp
        bind:checked={config.verbose}
        label="Verbose Logging"
        helpText="Show detailed logs in terminal"
      />
    </div>

    <!-- Advanced Settings -->
    <Accordion title="Advanced Context & Models">
      <div class="config-section">
        <LabelWithHelp
          id="clip-l"
          label="CLIP-L"
          helpText="Path to CLIP-L encoder"
        />
        <Input
          id="clip-l"
          label=""
          bind:value={config.clip_l}
          placeholder="Optional"
        />
      </div>
      <div class="config-section">
        <LabelWithHelp
          id="clip-g"
          label="CLIP-G"
          helpText="Path to CLIP-G encoder"
        />
        <Input
          id="clip-g"
          label=""
          bind:value={config.clip_g}
          placeholder="Optional"
        />
      </div>
      <div class="config-section">
        <LabelWithHelp
          id="t5xxl"
          label="T5XXL"
          helpText="Path to T5XXL encoder"
        />
        <Input
          id="t5xxl"
          label=""
          bind:value={config.t5xxl}
          placeholder="Optional"
        />
      </div>
      <div class="config-section">
        <LabelWithHelp
          id="control-net"
          label="ControlNet"
          helpText="Path to ControlNet model"
        />
        <Input
          id="control-net"
          label=""
          bind:value={config.control_net}
          placeholder="Optional"
        />
      </div>
      <div class="config-section">
        <LabelWithHelp
          id="lora-dir"
          label="LoRA Directory"
          helpText="Directory containing LoRA models"
        />
        <Input
          id="lora-dir"
          label=""
          bind:value={config.lora_model_dir}
          placeholder="Optional"
        />
      </div>
    </Accordion>

    <Accordion title="Advanced Generation">
      <div class="form-row">
        <div class="form-group half">
          <LabelWithHelp id="seed" label="Seed" helpText="-1 for random" />
          <Input id="seed" label="" type="number" bind:value={config.seed} />
        </div>
        <div class="form-group half">
          <LabelWithHelp
            id="batch-count"
            label="Batch Count"
            helpText="Images to generate"
          />
          <Input
            id="batch-count"
            label=""
            type="number"
            bind:value={config.batch_count}
          />
        </div>
      </div>
      <div class="form-row">
        <div class="form-group half">
          <LabelWithHelp
            id="guidance"
            label="Guidance"
            helpText="Guidance value"
          />
          <Input
            id="guidance"
            label=""
            type="number"
            step="0.1"
            bind:value={config.guidance}
          />
        </div>
        <div class="form-group half">
          <LabelWithHelp
            id="strength"
            label="Strength"
            helpText="Denoising strength"
          />
          <Input
            id="strength"
            label=""
            type="number"
            step="0.05"
            bind:value={config.strength}
          />
        </div>
      </div>
      <div class="config-section">
        <LabelWithHelp
          id="sampling-method"
          label="Sampling Method"
          helpText="euler, euler_a, heun, dpm2, etc."
        />
        <Input
          id="sampling-method"
          label=""
          bind:value={config.sampling_method}
          placeholder="euler_a"
        />
      </div>
      <div class="config-section">
        <LabelWithHelp
          id="scheduler"
          label="Scheduler"
          helpText="discrete, karras, etc."
        />
        <Input
          id="scheduler"
          label=""
          bind:value={config.scheduler}
          placeholder="discrete"
        />
      </div>
    </Accordion>

    <Accordion title="System & Paths">
      <div class="config-section">
        <LabelWithHelp
          id="output-path"
          label="Output Path"
          helpText="Directory for generated images"
        />
        <Input
          id="output-path"
          label=""
          bind:value={config.output_path}
          placeholder="./public"
        />
      </div>
      <div class="config-section">
        <LabelWithHelp
          id="models-path"
          label="Models Path"
          helpText="Base directory for models"
        />
        <Input
          id="models-path"
          label=""
          bind:value={config.models_path}
          placeholder="./sd_models"
        />
      </div>
      <div class="form-row">
        <div class="form-group half">
          <LabelWithHelp id="threads" label="Threads" helpText="-1 for auto" />
          <Input
            id="threads"
            label=""
            type="number"
            bind:value={config.threads}
          />
        </div>
        <div class="form-group half">
          <LabelWithHelp
            id="rng"
            label="RNG"
            helpText="Random Number Generator"
          />
          <Input
            id="rng"
            label=""
            bind:value={config.rng}
            placeholder="std_default"
          />
        </div>
      </div>
      <div class="config-section">
        <CheckboxWithHelp
          bind:checked={config.offload_to_cpu}
          label="Offload to CPU"
          helpText="Save VRAM by offloading weights"
        />
      </div>
      <div class="config-section">
        <CheckboxWithHelp
          bind:checked={config.color}
          label="Color Logging"
          helpText="Enable colored logs"
        />
      </div>
    </Accordion>
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
    transition: color 0.3s ease;
  }

  .config-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .config-section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-weight: 600;
    font-size: 1.1rem;
    color: var(--md-on-surface);
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 0.5rem;
    margin-bottom: 1rem;
  }

  .form-row {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .form-group {
    flex: 1;
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
      min-width: 100%;
      max-width: 100%;
    }
  }
</style>
