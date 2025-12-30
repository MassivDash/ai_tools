<script lang="ts">
  interface Props {
    id?: string
    label?: string
    value?: string
    options?: Array<{ value: string; label: string }>
    disabled?: boolean
    required?: boolean
    class?: string
    onchange?: (e: Event) => void
    row?: boolean
    [key: string]: any
  }

  let {
    id = '',
    label = '',
    value = $bindable(),
    options = [],
    disabled = false,
    required = false,
    class: extraClass = '',
    onchange,
    row = false,
    ...rest
  }: Props = $props()

  function handleChange(event: Event) {
    const target = event.target as HTMLSelectElement
    value = target.value
    if (onchange) {
      onchange(event)
    }
  }
</script>

<div class="dropdown-wrapper {extraClass}" class:row>
  {#if label}
    <label for={id} class="dropdown-label">{label}</label>
  {/if}
  <select
    {id}
    bind:value
    {disabled}
    {required}
    class="dropdown"
    onchange={handleChange}
    {...rest}
  >
    {#each options as option}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</div>

<style>
  .dropdown-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .dropdown-wrapper.row {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .dropdown-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: inherit;
  }

  .dropdown {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    font-size: 0.9rem;
    font-family: inherit;
    background-color: var(--bg-primary, white);
    color: var(--text-primary, #333);
    cursor: pointer;
    transition:
      border-color 0.2s,
      box-shadow 0.2s,
      background-color 0.3s ease,
      color 0.3s ease;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%23333' d='M6 9L1 4h10z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    padding-right: 2.5rem;
  }

  .dropdown:hover:not(:disabled) {
    border-color: var(--border-color-hover, #999);
  }

  .dropdown:focus {
    outline: none;
    border-color: var(--accent-color, #2196f3);
    box-shadow: 0 0 0 3px rgba(33, 150, 243, 0.1);
  }

  .dropdown:disabled {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
    opacity: 0.6;
  }

  /* Dark theme - update arrow icon color */
  :global(.dark) .dropdown {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%23fff' d='M6 9L1 4h10z'/%3E%3C/svg%3E");
  }
</style>
