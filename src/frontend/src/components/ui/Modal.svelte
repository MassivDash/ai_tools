<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  
  export let isOpen: boolean = false
  export let title: string = ''
  export let showCloseButton: boolean = true
  
  const dispatch = createEventDispatcher()
  
  function handleClose() {
    dispatch('close')
  }
  
  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose()
    }
  }
  
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      handleClose()
    }
  }
</script>

{#if isOpen}
  <div
    class="modal-overlay"
    role="button"
    tabindex="0"
    onclick={handleOverlayClick}
    onkeydown={handleKeydown}
  >
    <div
      class="modal-content"
      role="dialog"
      aria-labelledby="modal-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        {#if title}
          <h3 id="modal-title">{title}</h3>
        {/if}
        {#if showCloseButton}
          <button
            class="close-button"
            onclick={handleClose}
            aria-label="Close"
          >
            ×
          </button>
        {/if}
      </div>
      <div class="modal-body" onclick={(e) => e.stopPropagation()}>
        <slot />
      </div>
      {#if $$slots.footer}
        <div class="modal-footer" onclick={(e) => e.stopPropagation()}>
          <slot name="footer" />
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 1rem;
  }

  .modal-content {
    background-color: var(--bg-primary, white);
    border-radius: 8px;
    box-shadow: 0 4px 20px var(--shadow, rgba(0, 0, 0, 0.3));
    max-width: 700px;
    width: 100%;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    transition: background-color 0.3s ease;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 1px solid var(--border-color, #ddd);
    transition: border-color 0.3s ease;
  }

  .modal-header h3 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    font-size: 1.25rem;
    transition: color 0.3s ease;
  }

  .close-button {
    background: none;
    border: none;
    font-size: 2rem;
    cursor: pointer;
    color: var(--text-secondary, #666);
    line-height: 1;
    padding: 0;
    width: 2rem;
    height: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s, background-color 0.2s;
    border-radius: 4px;
  }

  .close-button:hover {
    color: var(--text-primary, #333);
    background-color: var(--bg-secondary, rgba(0, 0, 0, 0.05));
  }

  .modal-body {
    padding: 1.5rem;
    overflow-y: auto;
    flex: 1;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1.5rem;
    border-top: 1px solid var(--border-color, #ddd);
    transition: border-color 0.3s ease;
  }

  @media screen and (max-width: 768px) {
    .modal-content {
      max-width: 100%;
      max-height: 95vh;
    }
  }
</style>

