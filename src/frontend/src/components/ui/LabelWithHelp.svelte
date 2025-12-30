<script lang="ts">
  import HelpIcon from './HelpIcon.svelte'

  interface Props {
    id?: string
    label: string
    helpText: string
    class?: string
  }

  let { id = '', label, helpText, class: extraClass = '' }: Props = $props()
</script>

<div class="label-with-help {extraClass}">
  <label for={id} class="custom-label">{label}</label>
  <HelpIcon text={helpText} />
</div>

<style>
  .label-with-help {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    position: relative;
  }

  .custom-label {
    font-weight: 600;
    color: var(--text-primary, #333);
    font-size: 1rem;
    cursor: pointer;
    transition: color 0.3s ease;
  }
</style>
