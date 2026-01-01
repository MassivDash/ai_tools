<script lang="ts">
  import Input from '../../ui/Input.svelte'
  import CheckboxWithHelp from '../../ui/CheckboxWithHelp.svelte'
  import LabelWithHelp from '../../ui/LabelWithHelp.svelte'

  export let config: any
</script>

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
    id="vae-model"
    label="VAE"
    helpText="Path to standalone VAE model"
  />
  <Input
    id="vae-model"
    label=""
    bind:value={config.vae}
    placeholder="ae.safetensors"
  />
</div>

<div class="config-section">
  <LabelWithHelp
    id="llm-model"
    label="LLM / CLIP"
    helpText="Path to the LLM or text encoder (e.g. Qwen for SD3.5/Flux/Lumina)"
  />
  <Input
    id="llm-model"
    label=""
    bind:value={config.llm}
    placeholder="Qwen3-4B-Instruct-2507-Q8_0.gguf"
  />
</div>

<div class="config-section">
  <LabelWithHelp
    id="clip-l"
    label="CLIP-L"
    helpText="Path to the CLIP-L text encoder"
  />
  <Input
    id="clip-l"
    label=""
    bind:value={config.clip_l}
    placeholder="clip_l.safetensors"
  />
</div>

<div class="config-section">
  <LabelWithHelp
    id="clip-g"
    label="CLIP-G"
    helpText="Path to the CLIP-G text encoder"
  />
  <Input
    id="clip-g"
    label=""
    bind:value={config.clip_g}
    placeholder="clip_g.safetensors"
  />
</div>

<div class="config-section">
  <LabelWithHelp
    id="t5xxl"
    label="T5XXL"
    helpText="Path to the T5XXL text encoder"
  />
  <Input
    id="t5xxl"
    label=""
    bind:value={config.t5xxl}
    placeholder="t5xxl.safetensors"
  />
</div>

<div class="config-section">
  <CheckboxWithHelp
    bind:checked={config.diffusion_fa}
    label="Flash Attention"
    helpText="Use flash attention in the diffusion model"
  />

  <CheckboxWithHelp
    bind:checked={config.offload_to_cpu}
    label="Offload to CPU"
    helpText="Place weights in RAM to save VRAM"
  />
</div>

<style>
  .config-section {
    margin-bottom: 1.5rem;
  }
</style>
