<script lang="ts">
  import HelpIcon from '../../ui/HelpIcon.svelte'

  export let financialDataEnabled: boolean = false
  export let onToggle: (tool: string) => void
</script>

<div class="config-section">
  <div class="section-label">Tools:</div>
  <div class="tools-list">
    <label class="tool-checkbox">
      <input
        type="checkbox"
        checked={financialDataEnabled}
        onchange={() => onToggle('financial_data')}
        class="checkbox-input"
      />
      <span>My Financial Data</span>
      <HelpIcon
        text="Enable this tool to allow the agent to access your financial data including recent purchases and transactions."
      />
    </label>
  </div>
</div>

<style>
  .config-section {
    margin-bottom: 2rem;
  }

  .section-label {
    display: block;
    margin-bottom: 0.75rem;
    font-weight: 600;
    color: var(--text-primary, #333);
    font-size: 1rem;
    transition: color 0.3s ease;
  }

  .tools-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .tool-checkbox {
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
