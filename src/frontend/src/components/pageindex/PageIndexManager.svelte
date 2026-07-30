<script lang="ts">
  import { documents, selectedDocument } from '@stores/pageindex.ts'
  import DocumentList from './DocumentList.svelte'
  import UploadBook from './UploadBook.svelte'
  import TreeViewer from './TreeViewer.svelte'
  import PageSubHeader from '../ui/PageSubHeader.svelte'

  let documentListRef: DocumentList

  const handleUploaded = async () => {
    if (documentListRef) {
      await documentListRef.refresh()
    }
  }
</script>

<PageSubHeader title="PageIndex" icon="file-tree" />
<div class="pageindex-manager">
  <div class="manager-content">
    <div class="left-panel">
      <DocumentList bind:this={documentListRef} />
    </div>

    <div class="right-panel">
      <UploadBook on:uploaded={handleUploaded} />

      {#if $documents.length === 0}
        <div class="no-selection">
          <p>No books indexed, upload a PDF to start</p>
        </div>
      {:else if $selectedDocument}
        <TreeViewer document={$selectedDocument} />
      {:else}
        <div class="no-selection">
          <p>👈 Select a book from the left to inspect its structure</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .pageindex-manager {
    width: 100%;
    max-width: calc(100% - 5rem);
    margin: 0 auto;
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: visible;
  }

  .manager-content {
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    gap: 1.5rem;
    width: 100%;
    min-width: 0;
  }

  .left-panel,
  .right-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
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

  @media screen and (max-width: 768px) {
    .pageindex-manager {
      margin-top: 1rem;
    }
  }

  @media screen and (max-width: 1024px) {
    .manager-content {
      grid-template-columns: 1fr;
    }
  }
</style>
