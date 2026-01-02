<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import IconButton from '../ui/IconButton.svelte'

  interface SDImage {
    filename: string
    created: number
    path: string
    prompt?: string
    diffusion_model?: string
    width?: number
    height?: number
    steps?: number
    cfg_scale?: number
    seed?: number
    additional_info?: string
  }

  export let isGenerating = false

  let images: SDImage[] = []
  let loading = false
  let error = ''
  let fullscreenImage: SDImage | null = null

  const getBackendUrl = (path: string) => {
    const apiUrl = import.meta.env.PUBLIC_API_URL || 'http://localhost:8080/api'
    try {
      const url = new URL(apiUrl)
      return `${url.origin}${path}`
    } catch {
      return path
    }
  }

  export const refresh = async () => {
    loading = true
    error = ''
    try {
      const response = await axiosBackendInstance.get<{ images: SDImage[] }>(
        'sd-server/images'
      )
      images = response.data.images.sort((a, b) => b.created - a.created)
    } catch (err: any) {
      console.error('Failed to fetch images:', err)
      error = 'Failed to load images'
    } finally {
      loading = false
    }
  }

  const deleteImage = async (image: SDImage) => {
    if (!confirm(`Permanently delete this image?`)) return
    try {
      await axiosBackendInstance.delete(`sd-server/image/${image.filename}`)
      images = images.filter((img) => img.filename !== image.filename)
      if (fullscreenImage?.filename === image.filename) {
        fullscreenImage = null
      }
    } catch (err) {
      console.error('Failed to delete image:', err)
      alert('Failed to delete image')
    }
  }

  onMount(() => {
    refresh()
  })
</script>

<div class="gallery-container">
  <div class="gallery-header">
    <h3>Generated Images</h3>
    <IconButton variant="ghost" onclick={refresh} title="Refresh Gallery">
      <MaterialIcon name="refresh" width="20" height="20" />
    </IconButton>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="gallery-grid">
    {#if isGenerating}
      <div class="image-card skeleton">
        <div class="skeleton-image">
          <div class="spinner"></div>
          <span>Generating...</span>
        </div>
        <div class="skeleton-text"></div>
      </div>
    {/if}

    {#each images as image}
      <div class="image-card">
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div class="image-wrapper" onclick={() => (fullscreenImage = image)}>
          <img
            src={getBackendUrl(image.path)}
            alt={image.prompt || image.filename}
            loading="lazy"
          />
        </div>
        <div class="image-info">
          <span class="prompt-preview" title={image.prompt || image.filename}>
            {image.prompt || image.filename}
          </span>
          <span class="timestamp"
            >{new Date(image.created * 1000).toLocaleString()}</span
          >
        </div>
      </div>
    {/each}

    {#if images.length === 0 && !isGenerating && !loading}
      <div class="empty-state">
        <MaterialIcon name="image-off" width="48" height="48" />
        <p>No images generated yet.</p>
      </div>
    {/if}
  </div>

  {#if fullscreenImage}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="fullscreen-overlay" onclick={() => (fullscreenImage = null)}>
      <div class="overlay-content" onclick={(e) => e.stopPropagation()}>
        <div class="overlay-image-container">
          <img
            src={getBackendUrl(fullscreenImage.path)}
            alt={fullscreenImage.prompt || fullscreenImage.filename}
          />
        </div>
        <div class="overlay-sidebar">
          <div class="sidebar-header">
            <h4>Details</h4>
            <IconButton
              variant="ghost"
              onclick={() => (fullscreenImage = null)}
              title="Close"
            >
              <MaterialIcon name="close" width="20" height="20" />
            </IconButton>
          </div>

          <div class="detail-row">
            <span class="label">Model</span>
            <span class="value"
              >{fullscreenImage.diffusion_model || 'Unknown'}</span
            >
          </div>
          {#if fullscreenImage.prompt}
            <div class="detail-row prompt">
              <span class="label">Prompt</span>
              <div class="value prompt-text">{fullscreenImage.prompt}</div>
            </div>
          {/if}
          <div class="detail-grid">
            <div class="detail-item">
              <span class="label">Size</span>
              <span class="value"
                >{fullscreenImage.width} x {fullscreenImage.height}</span
              >
            </div>
            {#if fullscreenImage.steps}
              <div class="detail-item">
                <span class="label">Steps</span>
                <span class="value">{fullscreenImage.steps}</span>
              </div>
            {/if}
            {#if fullscreenImage.cfg_scale}
              <div class="detail-item">
                <span class="label">CFG</span>
                <span class="value">{fullscreenImage.cfg_scale}</span>
              </div>
            {/if}
            {#if fullscreenImage.seed !== undefined}
              <div class="detail-item">
                <span class="label">Seed</span>
                <span class="value">{fullscreenImage.seed}</span>
              </div>
            {/if}
          </div>

          <div class="actions">
            <button
              class="delete-btn"
              onclick={() => fullscreenImage && deleteImage(fullscreenImage)}
            >
              <MaterialIcon name="delete" width="20" height="20" /> Delete
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .gallery-container {
    width: 100%;
    padding: 1rem;
    box-sizing: border-box;
  }

  .gallery-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .gallery-header h3 {
    margin: 0;
    font-size: 1.25rem;
    color: var(--md-on-surface);
  }

  .gallery-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .image-card {
    background-color: var(--md-surface);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
    transition:
      transform 0.2s,
      box-shadow 0.2s;
    border: 1px solid var(--md-outline-variant);
    display: flex;
    flex-direction: column;
  }

  .image-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .image-wrapper {
    width: 100%;
    aspect-ratio: 1;
    background-color: var(--md-surface-variant);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    cursor: pointer;
  }

  .image-wrapper img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.3s;
  }

  .image-card:hover .image-wrapper img {
    transform: scale(1.05);
  }

  .image-info {
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    background-color: var(--md-surface);
  }

  .prompt-preview {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--md-on-surface);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }

  .timestamp {
    font-size: 0.75rem;
    color: var(--md-on-surface-variant);
  }

  .empty-state {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--md-on-surface-variant);
    opacity: 0.6;
    gap: 1rem;
  }

  /* Skeleton */
  .skeleton .skeleton-image {
    width: 100%;
    aspect-ratio: 1;
    background-color: var(--md-surface-variant);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    color: var(--md-on-surface-variant);
  }

  .skeleton .skeleton-text {
    height: 40px;
    background-color: var(--md-surface);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 3px solid var(--md-primary);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    color: var(--md-error);
    padding: 1rem;
    background-color: var(--md-error-container);
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  /* Overlay */
  .fullscreen-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(0, 0, 0, 0.85);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 0.2s ease-out;
    padding: 2rem;
    box-sizing: border-box;
  }

  .overlay-content {
    display: flex;
    background-color: var(--md-surface);
    border-radius: 12px;
    overflow: hidden;
    max-width: 95vw;
    max-height: 90vh;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
    animation: scaleIn 0.2s ease-out;
  }

  .overlay-image-container {
    background-color: #000;
    flex-grow: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 400px;
    overflow: hidden;
  }

  .overlay-image-container img {
    max-width: 100%;
    max-height: 90vh;
    object-fit: contain;
    display: block;
  }

  .overlay-sidebar {
    width: 320px;
    padding: 1.5rem;
    background-color: var(--md-surface);
    color: var(--md-on-surface);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    border-left: 1px solid var(--md-outline-variant);
    flex-shrink: 0;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .sidebar-header h4 {
    margin: 0;
    color: var(--md-primary);
    font-size: 1.1rem;
  }

  .detail-row {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .detail-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .label {
    font-weight: 600;
    color: var(--md-on-surface-variant);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .value {
    color: var(--md-on-surface);
    font-size: 0.9rem;
    word-break: break-word;
  }

  .prompt-text {
    background-color: var(--md-surface-variant);
    padding: 0.75rem;
    border-radius: 8px;
    color: var(--md-on-surface);
    font-size: 0.9rem;
    line-height: 1.4;
    max-height: 150px;
    overflow-y: auto;
    white-space: pre-wrap;
  }

  .actions {
    margin-top: auto;
    padding-top: 1.5rem;
  }

  .delete-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.75rem;
    background-color: var(--md-error-container);
    color: var(--md-error);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
    transition: background-color 0.2s;
  }

  .delete-btn:hover {
    background-color: #ffdad6;
  }

  @media (max-width: 850px) {
    .overlay-content {
      flex-direction: column;
      width: 95vw;
      max-height: 95vh;
    }
    .overlay-image-container {
      width: 100%;
      min-width: auto;
      height: 50vh;
    }
    .overlay-sidebar {
      width: 100%;
      height: 40vh;
      border-left: none;
      border-top: 1px solid var(--md-outline-variant);
    }
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  @keyframes scaleIn {
    from {
      transform: scale(0.98);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>
