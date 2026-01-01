<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import CheckboxWithHelp from '../../ui/CheckboxWithHelp.svelte'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'
  import Accordion from '../../ui/Accordion.svelte'
  import type { SDConfig } from '../../../validation/stableDiffusion'

  export let config: SDConfig
</script>

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
      <Input id="threads" label="" type="number" bind:value={config.threads} />
    </div>
    <div class="form-group half">
      <LabelWithHelp id="rng" label="RNG" helpText="Random Number Generator" />
      <Input
        id="rng"
        label=""
        bind:value={config.rng}
        placeholder="std_default"
      />
    </div>
  </div>
  <div class="config-section">
    <h4>Low VRAM / Memory Optimizations</h4>
    <div class="checkbox-grid">
      <CheckboxWithHelp
        bind:checked={config.offload_to_cpu}
        label="Offload to CPU"
        helpText="Offload model weights to RAM"
      />
      <CheckboxWithHelp
        bind:checked={config.control_net_cpu}
        label="ControlNet to CPU"
        helpText="Keep ControlNet in RAM"
      />
      <CheckboxWithHelp
        bind:checked={config.clip_on_cpu}
        label="CLIP to CPU"
        helpText="Keep CLIP in RAM"
      />
      <CheckboxWithHelp
        bind:checked={config.vae_on_cpu}
        label="VAE to CPU"
        helpText="Keep VAE in RAM"
      />
      <CheckboxWithHelp
        bind:checked={config.vae_tiling}
        label="VAE Tiling"
        helpText="Process VAE in tiles (saves VRAM)"
      />
    </div>
  </div>
  <div class="config-section">
    <CheckboxWithHelp
      bind:checked={config.color}
      label="Color Logging"
      helpText="Enable colored logs"
    />
  </div>
</Accordion>

<style>
  .config-section {
    margin-bottom: 1.5rem;
  }
  .form-row {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  .form-group {
    flex: 1;
  }
  .checkbox-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
    margin-top: 0.5rem;
  }
  h4 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }
</style>
