<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    stage: Snippet
    controls: Snippet
    contestants: Snippet
  }

  let { stage, controls, contestants }: Props = $props()
</script>

<div class="presenter-dashboard">
  <div class="stage-section">
    {@render stage()}
  </div>

  <div class="dashboard-grid">
    <div class="controls-section">
      {@render controls()}
    </div>

    <div class="contestants-section">
      {@render contestants()}
    </div>
  </div>
</div>

<style>
  .presenter-dashboard {
    text-align: left;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    height: 100%;
    color: var(--text-primary);
  }

  .stage-section {
    flex-grow: 1;
    min-height: 400px;
    display: flex;
    flex-direction: column;
  }

  .dashboard-grid {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 1.5rem;
  }

  .controls-section,
  .contestants-section {
    height: 100%;
  }
</style>
