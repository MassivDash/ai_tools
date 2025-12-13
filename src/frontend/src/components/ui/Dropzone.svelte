<script lang="ts">
  import Button from './Button.svelte'
  import { createEventDispatcher } from 'svelte'

  export let accept: string = '*'
  export let multiple: boolean = true
  export let disabled: boolean = false
  export let buttonText: string = 'Browse Files'
  export let hint: string = ''

  const dispatch = createEventDispatcher<{
    files: File[]
  }>()

  let dragActive = false
  let fileInputId = `file-input-${Math.random().toString(36).substr(2, 9)}`
  let fileInput: HTMLInputElement

  const handleDrag = (e: DragEvent) => {
    if (disabled) return
    e.preventDefault()
    e.stopPropagation()
    if (e.type === 'dragenter' || e.type === 'dragover') {
      dragActive = true
    } else if (e.type === 'dragleave') {
      dragActive = false
    }
  }

  const handleDrop = (e: DragEvent) => {
    if (disabled) return
    e.preventDefault()
    e.stopPropagation()
    dragActive = false

    if (e.dataTransfer?.files) {
      handleFiles(Array.from(e.dataTransfer.files))
    }
  }

  const handleFileSelect = (e: Event) => {
    const target = e.target as HTMLInputElement
    if (target.files) {
      handleFiles(Array.from(target.files))
    }
  }

  const handleFiles = (files: File[]) => {
    if (files.length > 0) {
      dispatch('files', files)
    }
  }

  const handleDropzoneClick = (e: MouseEvent) => {
    if (disabled) return
    // Don't trigger if clicking the button or label (they handle it themselves)
    const target = e.target as HTMLElement
    if (target.closest('.file-input-label') || target.closest('button')) {
      return
    }
    fileInput?.click()
  }

  const handleButtonClick = (e: MouseEvent) => {
    if (disabled) return
    e.stopPropagation() // Prevent dropzone click handler
    fileInput?.click()
  }
</script>

<div
  class="dropzone"
  class:active={dragActive}
  class:disabled={disabled}
  onclick={handleDropzoneClick}
  ondragenter={handleDrag}
  ondragover={handleDrag}
  ondragleave={handleDrag}
  ondrop={handleDrop}
>
  <div class="dropzone-content">
    <div class="dropzone-icon">
      <svg
        width="48"
        height="48"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
        <polyline points="17 8 12 3 7 8" />
        <line x1="12" y1="3" x2="12" y2="15" />
      </svg>
    </div>
    <p class="dropzone-text">
      Drag and drop files here, or
    </p>
    <label for={fileInputId} class="file-input-label" onclick={(e) => e.stopPropagation()}>
      <Button type="button" variant="primary" disabled={disabled} onclick={handleButtonClick}>
        {buttonText}
      </Button>
      <input
        bind:this={fileInput}
        id={fileInputId}
        type="file"
        {multiple}
        {accept}
        onchange={handleFileSelect}
        {disabled}
        style="display: none;"
      />
    </label>
    {#if hint}
      <p class="dropzone-hint">{hint}</p>
    {/if}
  </div>
</div>

<style>
  .dropzone {
    border: 2px dashed var(--border-color);
    border-radius: 8px;
    padding: 3rem 2rem;
    text-align: center;
    transition:
      all 0.2s ease,
      background-color 0.3s ease,
      border-color 0.3s ease;
    background: var(--bg-primary);
    cursor: pointer;
  }

  .dropzone:hover:not(.disabled) {
    border-color: var(--border-color-hover);
    background: var(--bg-secondary);
  }

  .dropzone.active {
    border-color: var(--accent-color, #4a90e2);
    background: var(--bg-secondary);
    box-shadow: 0 0 0 4px rgba(74, 144, 226, 0.1);
  }

  .dropzone.disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .dropzone-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .dropzone-icon {
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }

  .dropzone.active .dropzone-icon {
    color: var(--accent-color, #4a90e2);
  }

  .dropzone-text {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
    transition: color 0.3s ease;
  }

  .file-input-label {
    cursor: pointer;
  }

  .dropzone.disabled .file-input-label {
    cursor: not-allowed;
  }

  .dropzone-hint {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text-tertiary);
    transition: color 0.3s ease;
  }
</style>

