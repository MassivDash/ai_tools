<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ChromaDBHealthResponse, ChromaDBResponse } from '../../types/chromadb.ts'
  import CollectionList from './CollectionList.svelte'
  import CreateCollection from './CreateCollection.svelte'
  import DocumentUpload from './DocumentUpload.svelte'
  import QueryInterface from './QueryInterface.svelte'

  let healthStatus: ChromaDBHealthResponse | null = null
  let selectedCollection: string | null = null
  let collectionListRef: CollectionList

  const checkHealth = async () => {
    try {
      console.log('🏥 Checking ChromaDB health...')
      const response = await axiosBackendInstance.get<ChromaDBResponse<ChromaDBHealthResponse>>(
        'chromadb/health'
      )
      if (response.data.success && response.data.data) {
        healthStatus = response.data.data
        console.log('✅ ChromaDB health:', healthStatus)
      }
    } catch (err: any) {
      console.error('❌ Error checking ChromaDB health:', err)
      healthStatus = {
        status: 'unhealthy',
        version: 'unknown',
        chromadb: { connected: false }
      }
    }
  }

  const handleCollectionCreated = () => {
    if (collectionListRef) {
      collectionListRef.refresh()
    }
  }

  const handleCollectionSelected = (event: CustomEvent<{ name: string }>) => {
    selectedCollection = event.detail.name
  }

  const handleDocumentUploaded = () => {
    if (collectionListRef) {
      collectionListRef.refresh()
    }
  }

  onMount(() => {
    checkHealth()
  })
</script>

<div class="chromadb-manager">
  <div class="status-bar">
    <div class="health-status">
      <span class="status-indicator" class:healthy={healthStatus?.chromadb.connected} class:unhealthy={!healthStatus?.chromadb.connected}>
        {healthStatus?.chromadb.connected ? '🟢' : '🔴'}
      </span>
      <span class="status-text">
        {healthStatus?.chromadb.connected ? 'ChromaDB Connected' : 'ChromaDB Disconnected'}
      </span>
      {#if healthStatus}
        <span class="version">v{healthStatus.version}</span>
      {/if}
    </div>
    <button class="refresh-btn" onclick={checkHealth}>🔄 Refresh</button>
  </div>

  <div class="manager-content">
    <div class="left-panel">
      <CreateCollection on:created={handleCollectionCreated} />
      <CollectionList bind:this={collectionListRef} on:select={(e) => handleCollectionSelected(e.detail.name)} />
    </div>

    <div class="right-panel">
      {#if selectedCollection}
        <div class="selected-collection">
          <h2>Collection: {selectedCollection}</h2>
          <DocumentUpload {selectedCollection} on:uploaded={handleDocumentUploaded} />
          <QueryInterface {selectedCollection} />
        </div>
      {:else}
        <div class="no-selection">
          <p>👈 Select a collection from the left to upload documents or search</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .chromadb-manager {
    width: 100%;
    max-width: 1400px;
    margin: 0 auto;
  }

  .status-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: var(--bg-primary, white);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    margin-bottom: 1.5rem;
  }

  .health-status {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .status-indicator {
    font-size: 1.2rem;
  }

  .status-text {
    font-weight: 600;
    color: var(--text-primary, #100f0f);
  }

  .version {
    font-size: 0.85rem;
    color: var(--text-secondary, #666);
    padding: 0.25rem 0.5rem;
    background: var(--bg-secondary, #f5f5f5);
    border-radius: 4px;
  }

  .refresh-btn {
    background: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    padding: 0.5rem 1rem;
    cursor: pointer;
    font-size: 0.9rem;
    transition: all 0.2s;
  }

  .refresh-btn:hover {
    background: var(--bg-tertiary, #e8e8e8);
  }

  .manager-content {
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    gap: 1.5rem;
  }

  .left-panel,
  .right-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .selected-collection {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .selected-collection h2 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    padding-bottom: 1rem;
    border-bottom: 2px solid var(--border-color, #ddd);
  }

  .no-selection {
    padding: 3rem;
    text-align: center;
    color: var(--text-secondary, #666);
    background: var(--bg-primary, white);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
  }

  @media screen and (max-width: 1024px) {
    .manager-content {
      grid-template-columns: 1fr;
    }
  }
</style>

