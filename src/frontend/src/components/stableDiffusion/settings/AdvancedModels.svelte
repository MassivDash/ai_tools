<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'
  import Accordion from '../../ui/Accordion.svelte'
  import type { SDConfig } from '../../../validation/stableDiffusion'

  export let config: SDConfig
</script>

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
    <LabelWithHelp id="t5xxl" label="T5XXL" helpText="Path to T5XXL encoder" />
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

<style>
  .config-section {
    margin-bottom: 1.5rem;
  }
</style>
