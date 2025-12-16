<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ChromaDBHealthResponse, ChromaDBResponse } from '@types'
  import { collections, selectedCollection } from '@stores/chromadb.ts'
  import CollectionList from './collectionList/CollectionList.svelte'
  import DocumentUpload from './DocumentUpload.svelte'
  import QueryInterface from './QueryInterface.svelte'
  import ChromaDBConfig from './ChromaDBConfig.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import Button from '../ui/Button.svelte'

  let healthStatus: ChromaDBHealthResponse | null = null
  let collectionListRef: CollectionList
  let configPanelOpen = false

  const checkHealth = async () => {
    try {
      const response =
        await axiosBackendInstance.get<
          ChromaDBResponse<ChromaDBHealthResponse>
        >('chromadb/health')
      if (response.data.success && response.data.data) {
        healthStatus = response.data.data
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

  const handleDocumentUploaded = async () => {
    if (collectionListRef) {
      await collectionListRef.refresh()
      // Update the selected collection with the latest count from the collections store
      selectedCollection.update((current) => {
        if (current) {
          const updated = $collections.find(
            (c) => c.id === current.id || c.name === current.name
          )
          if (updated) {
            return updated
          }
        }
        return current
      })
    }
  }

  onMount(() => {
    checkHealth()
  })
</script>

<div class="chromadb-manager">
  <PageSubHeader title="ChromaDB">
    {#snippet leftContent()}
      <div class="health-status">
        <span
          class="status-indicator"
          class:healthy={healthStatus?.chromadb.connected}
          class:unhealthy={!healthStatus?.chromadb.connected}
        >
          {healthStatus?.chromadb.connected ? '🟢' : '🔴'}
        </span>
        <span class="status-text">
          {healthStatus?.chromadb.connected
            ? 'ChromaDB Connected'
            : 'ChromaDB Disconnected'}
        </span>
        {#if healthStatus}
          <span class="version">v{healthStatus.version}</span>
        {/if}
      </div>
    {/snippet}
    {#snippet actions()}
      <Button
        variant="info"
        class="button-icon-only"
        onclick={() => (configPanelOpen = true)}
        title="Configure Embedding Models"
      >
        <MaterialIcon name="cog" width="32" height="32" />
      </Button>
      <Button
        variant="info"
        class="button-icon-only"
        onclick={checkHealth}
        title="Refresh Health Status"
      >
        <MaterialIcon name="refresh" width="32" height="32" />
      </Button>
    {/snippet}
  </PageSubHeader>

  <div class="content-area" class:has-config={configPanelOpen}>
    <div class="manager-content">
      <div class="left-panel">
        <CollectionList bind:this={collectionListRef} />
      </div>

      <div class="right-panel">
        {#if $collections.length === 0}
          <div class="no-selection">
            <p>No collections, add collection to start</p>
          </div>
        {:else if $selectedCollection}
          <div class="selected-collection">
            <h2>Collection: {$selectedCollection.name}</h2>
            <DocumentUpload
              selectedCollection={$selectedCollection.name}
              on:uploaded={handleDocumentUploaded}
            />
            {#if $selectedCollection.count !== undefined && $selectedCollection.count > 0}
              <QueryInterface selectedCollection={$selectedCollection.name} />
            {/if}
          </div>
        {:else}
          <div class="no-selection">
            <p>
              👈 Select a collection from the left to upload documents or search
            </p>
          </div>
        {/if}
      </div>
    </div>
  </div>
  <ChromaDBConfig
    isOpen={configPanelOpen}
    onClose={() => (configPanelOpen = false)}
    onSave={() => {
      // Config saved, no action needed
    }}
  />
</div>

<style>
  .chromadb-manager {
    width: 100%;
    max-width: 1400px;
    margin: 0 auto;
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: visible;
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
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .version {
    font-size: 0.85rem;
    color: var(--text-secondary);
    padding: 0.25rem 0.5rem;
    background: var(--bg-secondary);
    border-radius: 4px;
    transition:
      color 0.3s ease,
      background-color 0.3s ease;
  }

  .content-area {
    flex: 1;
    position: relative;
    min-height: 60vh;
    overflow: hidden;
    width: 100%;
    transition:
      margin-right 0.3s ease-in-out,
      transform 0.3s ease-in-out;
    margin-right: 0;
  }

  .content-area.has-config {
    margin-right: 70vw;
    transform: translateX(0);
  }

  @media (min-width: 1401px) {
    .content-area.has-config {
      margin-right: 980px;
    }
  }

  .manager-content {
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    gap: 1.5rem;
    width: 100%;
    min-width: 0;
    transition: margin-right 0.3s ease-in-out;
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
    color: var(--text-primary);
    padding-bottom: 1rem;
    border-bottom: 2px solid var(--border-color);
    transition:
      color 0.3s ease,
      border-color 0.3s ease;
  }

  .no-selection {
    padding: 3rem;
    text-align: center;
    color: var(--text-secondary);
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  @media screen and (max-width: 1024px) {
    .manager-content {
      grid-template-columns: 1fr;
    }

    .content-area.has-config {
      margin-right: 0 !important;
    }

    :global(.config-panel) {
      width: 100vw !important;
      max-width: 100vw !important;
      top: 80px !important;
      height: calc(100vh - 80px) !important;
    }
  }
</style>
