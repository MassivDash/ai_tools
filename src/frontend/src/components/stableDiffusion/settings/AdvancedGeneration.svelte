<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'
  import Accordion from '../../ui/Accordion.svelte'
  import type { SDConfig } from '../../../validation/stableDiffusion'

  export let config: SDConfig
</script>

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
      <LabelWithHelp id="guidance" label="Guidance" helpText="Guidance value" />
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
</style>
