<script lang="ts">
  interface Props {
    text?: string
  }

  let { text = '' }: Props = $props()
  let showTooltip = $state(false)

  const handleMouseEnter = () => {
    showTooltip = true
  }

  const handleMouseLeave = () => {
    showTooltip = false
  }
</script>

<div class="help-icon-wrapper" on:mouseenter={handleMouseEnter} on:mouseleave={handleMouseLeave}>
  <span class="help-icon" aria-label="Help">?</span>
  {#if showTooltip && text}
    <div class="tooltip">{text}</div>
  {/if}
</div>

<style>
  .help-icon-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    margin-left: 0.5rem;
  }

  .help-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 50%;
    background-color: var(--bg-tertiary, #e0e0e0);
    color: var(--text-secondary, #666);
    font-size: 0.75rem;
    font-weight: 600;
    cursor: help;
    transition: background-color 0.2s ease, color 0.2s ease;
  }

  .help-icon:hover {
    background-color: var(--accent-color, #2196f3);
    color: white;
  }

  .tooltip {
    position: absolute;
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    margin-bottom: 0.5rem;
    background-color: var(--bg-primary, #333);
    color: var(--text-primary, #fff);
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    font-size: 0.85rem;
    line-height: 1.4;
    max-width: 300px;
    min-width: 200px;
    z-index: 1000;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    white-space: normal;
    word-wrap: break-word;
    pointer-events: none;
  }

  .tooltip::after {
    content: '';
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border: 6px solid transparent;
    border-top-color: var(--bg-primary, #333);
  }
</style>

