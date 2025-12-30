<script lang="ts">
  import HelpIcon from './HelpIcon.svelte'

  interface Props {
    checked: boolean
    label: string
    helpText?: string
    class?: string
    onchange?: (e: Event) => void
  }

  let {
    checked = $bindable(),
    label,
    helpText = '',
    class: extraClass = '',
    onchange
  }: Props = $props()
</script>

<div class="checkbox-with-help {extraClass}">
  <div class="checkbox-wrapper">
    <label class="checkbox-label">
      <input type="checkbox" bind:checked {onchange} class="checkbox-input" />
      <span>{label}</span>
    </label>
    <HelpIcon text={helpText} />
  </div>
</div>

<style>
  .checkbox-with-help {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    position: relative;
  }

  .checkbox-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-weight: 600;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .checkbox-input {
    width: 1.25rem;
    height: 1.25rem;
    cursor: pointer;
    accent-color: var(--accent-color, #2196f3);
  }
</style>
