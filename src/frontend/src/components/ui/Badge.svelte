<script lang="ts">
  interface Props {
    variant?:
      | 'primary'
      | 'secondary'
      | 'success'
      | 'danger'
      | 'info'
      | 'warning'
    class?: string
    children?: import('svelte').Snippet
  }

  let { variant = 'primary', class: className = '', children }: Props = $props()
</script>

<span class="badge badge-{variant} {className}">
  {#if children}
    {@render children()}
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    padding: 0.25rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 600;
    line-height: 1;
    border-radius: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    white-space: nowrap;
  }

  .badge-primary {
    background-color: var(--accent-color, #2196f3);
    color: white;
  }

  .badge-secondary {
    background-color: var(--bg-tertiary, #e0e0e0);
    color: var(--text-primary, #100f0f);
  }

  .badge-success {
    background-color: #4caf50;
    color: white;
  }

  .badge-danger {
    background-color: #f44336;
    color: white;
  }

  .badge-info {
    background-color: #2196f3;
    color: white;
  }

  .badge-warning {
    background-color: #ff9800;
    color: white;
  }
</style>
