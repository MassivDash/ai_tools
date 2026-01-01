<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import CheckboxWithHelp from '../../ui/CheckboxWithHelp.svelte'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'
  import type { SDConfig } from '../../../schemas/stableDiffusion'

  export let config: SDConfig
</script>

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
  <LabelWithHelp id="vae" label="VAE" helpText="Path to standalone VAE model" />
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
    <LabelWithHelp id="cfg-scale" label="CFG Scale" helpText="Guidance scale" />
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

<style>
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
</style>
