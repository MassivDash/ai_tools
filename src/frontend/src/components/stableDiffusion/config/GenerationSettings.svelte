<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'

  export let config: any
</script>

<div class="form-row">
  <div class="form-group half">
    <LabelWithHelp
      id="height"
      label="Height"
      helpText="Image height in pixels"
    />
    <Input id="height" label="" type="number" bind:value={config.height} />
  </div>
  <div class="form-group half">
    <LabelWithHelp id="width" label="Width" helpText="Image width in pixels" />
    <Input id="width" label="" type="number" bind:value={config.width} />
  </div>
</div>

<div class="form-row">
  <div class="form-group half">
    <LabelWithHelp id="steps" label="Steps" helpText="Number of sample steps" />
    <Input id="steps" label="" type="number" bind:value={config.steps} />
  </div>
  <div class="form-group half">
    <LabelWithHelp
      id="cfg-scale"
      label="CFG Scale"
      helpText="Unconditional guidance scale"
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
  <LabelWithHelp
    id="sampling-method"
    label="Sampling Method"
    helpText="euler, euler_a, heun, dpm2, dpm++2s_a, etc."
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
    helpText="discrete, karras, exponential, etc."
  />
  <Input
    id="scheduler"
    label=""
    bind:value={config.scheduler}
    placeholder="discrete"
  />
</div>

<div class="form-row">
  <div class="form-group half">
    <LabelWithHelp id="seed" label="Seed" helpText="RNG seed (-1 for random)" />
    <Input id="seed" label="" type="number" bind:value={config.seed} />
  </div>
  <div class="form-group half">
    <LabelWithHelp
      id="batch-count"
      label="Batch Count"
      helpText="Number of images to generate"
    />
    <Input
      id="batch-count"
      label=""
      type="number"
      bind:value={config.batch_count}
    />
  </div>
</div>

<style>
  .config-section {
    margin-bottom: 1.5rem;
  }

  .form-row {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .half {
    flex: 1;
  }
</style>
