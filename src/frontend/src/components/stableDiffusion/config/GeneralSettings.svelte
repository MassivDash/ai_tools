<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'
  import CheckboxWithHelp from '../../ui/CheckboxWithHelp.svelte'

  export let config: any
</script>

<div class="config-section">
  <LabelWithHelp
    id="models-path"
    label="Models Path"
    helpText="Base directory for your models. sd-cli commands will run from this directory."
  />
  <Input
    id="models-path"
    label=""
    bind:value={config.models_path}
    placeholder="/path/to/models"
  />
</div>

<div class="config-section">
  <LabelWithHelp
    id="output-path"
    label="Output Path"
    helpText="Path to write result image to. Can use printf-style %d format specifiers."
  />
  <Input
    id="output-path"
    label=""
    bind:value={config.output_path}
    placeholder="./images/output.png"
  />
</div>

<div class="config-section">
  <div class="checkbox-row">
    <CheckboxWithHelp
      bind:checked={config.verbose}
      label="Verbose"
      helpText="Print extra info"
    />
    <CheckboxWithHelp
      bind:checked={config.color}
      label="Color"
      helpText="Colors the logging tags"
    />
  </div>
</div>

<style>
  .config-section {
    margin-bottom: 1.5rem;
  }
  .checkbox-row {
    display: flex;
    gap: 1rem;
  }
</style>
