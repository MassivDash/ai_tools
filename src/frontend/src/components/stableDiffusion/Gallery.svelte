<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import MaterialIcon from '../ui/MaterialIcon.svelte'

  interface SDImage {
    filename: string
    created: number
    path: string
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
    } catch (e) {
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

  onMount(() => {
    refresh()
  })
</script>

<div class="gallery-container">
  <div class="gallery-header">
    <h3>Generated Images</h3>
    <button class="refresh-btn" onclick={refresh} title="Refresh Gallery">
      <MaterialIcon name="refresh" width="20" height="20" />
    </button>
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
            alt={image.filename}
            loading="lazy"
          />
        </div>
        <div class="image-info">
          <span class="filename" title={image.filename}>{image.filename}</span>
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
      <img
        src={getBackendUrl(fullscreenImage.path)}
        alt={fullscreenImage.filename}
        onclick={(e) => e.stopPropagation()}
      />
      <button class="close-btn" onclick={() => (fullscreenImage = null)}>
        <MaterialIcon name="close" width="32" height="32" />
      </button>
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

  .refresh-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--md-primary);
    padding: 0.5rem;
    border-radius: 50%;
    transition: background-color 0.3s;
  }

  .refresh-btn:hover {
    background-color: var(--md-surface-variant);
  }

  .gallery-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 1.5rem;
  }

  .image-card {
    background-color: var(--md-surface);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    transition:
      transform 0.2s,
      box-shadow 0.2s;
    border: 1px solid var(--md-outline-variant);
    display: flex;
    flex-direction: column;
  }

  .image-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .image-wrapper {
    width: 100%;
    aspect-ratio: 1; /* Square thumbnails */
    background-color: #f0f0f0;
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

  .filename {
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--md-on-surface);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
  }

  /* Skeleton Loader */
  .skeleton .skeleton-image {
    width: 100%;
    height: 100%;
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
  /* Fullscreen Overlay */
  .fullscreen-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(0, 0, 0, 0.9);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    animation: fadeIn 0.2s ease-out;
  }

  .fullscreen-overlay img {
    max-width: 90%;
    max-height: 90vh;
    object-fit: contain;
    border-radius: 4px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
    cursor: default;
    animation: scaleIn 0.2s ease-out;
  }

  .close-btn {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: none;
    border: none;
    color: white;
    cursor: pointer;
    padding: 0.5rem;
    border-radius: 50%;
    transition: background-color 0.2s;
  }

  .close-btn:hover {
    background-color: rgba(255, 255, 255, 0.1);
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
      transform: scale(0.95);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>
