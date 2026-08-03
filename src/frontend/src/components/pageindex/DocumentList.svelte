<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { PageIndexDocument, PageIndexListResponse } from '@types'
  import { documents, selectedDocument } from '@stores/pageindex.ts'
  import DocumentCard from './DocumentCard.svelte'
  import IconButton from '@ui/IconButton.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'

  let loading = false
  let error = ''
  let pollInterval: ReturnType<typeof setInterval> | null = null

  const POLL_INTERVAL_MS = 5000

  const loadDocuments = async () => {
    loading = true
    error = ''
    try {
      const response = await axiosBackendInstance.get<PageIndexListResponse>(
        'pageindex/documents'
      )
      if (response.data.success) {
        documents.set(response.data.documents)

        // Update selectedDocument with latest data if it's still selected
        selectedDocument.update((current) => {
          if (current) {
            const updated = response.data.documents.find(
              (d) => d.id === current.id
            )
            if (updated) {
              return updated
            }
          }
          return current
        })

        updatePolling(response.data.documents)
      } else {
        error = response.data.error || 'Failed to load documents'
      }
    } catch (err: any) {
      console.error('Error loading documents:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load documents'
    } finally {
      loading = false
    }
  }

  const updatePolling = (docs: PageIndexDocument[]) => {
    const hasProcessing = docs.some((d) => d.status === 'processing')
    if (hasProcessing && !pollInterval) {
      pollInterval = setInterval(loadDocuments, POLL_INTERVAL_MS)
    } else if (!hasProcessing && pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }
  }

  const handleDocumentSelect = (document: PageIndexDocument) => {
    selectedDocument.set(document)
  }

  const handleDocumentDelete = async (id: string, title: string) => {
    if (!confirm(`Are you sure you want to delete "${title}"?`)) {
      return
    }

    try {
      const response = await axiosBackendInstance.delete<{
        success: boolean
        error?: string
      }>(`pageindex/documents/${id}`)
      if (response.data.success) {
        let wasSelected = false
        selectedDocument.update((current) => {
          wasSelected = current?.id === id
          return current
        })

        documents.update((docs) => {
          const updated = docs.filter((d) => d.id !== id)
          if (wasSelected) {
            selectedDocument.set(updated.length > 0 ? updated[0] : null)
          }
          return updated
        })

        await loadDocuments()
      } else {
        error = response.data.error || 'Failed to delete document'
      }
    } catch (err: any) {
      console.error('Error deleting document:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to delete document'
    }
  }

  onMount(() => {
    loadDocuments()
  })

  onDestroy(() => {
    if (pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }
  })

  // Expose refresh function
  export const refresh = loadDocuments
</script>

<div class="document-list">
  <div class="header">
    <h2>Books</h2>
    <div class="header-actions">
      <IconButton
        variant="info"
        onclick={loadDocuments}
        disabled={loading}
        title={loading ? 'Loading...' : 'Refresh Documents'}
      >
        <MaterialIcon name="refresh" width="24" height="24" />
      </IconButton>
    </div>
  </div>

  {#if error}
    <div class="error-message">{error}</div>
  {/if}

  {#if loading && $documents.length === 0}
    <div class="loading">Loading documents...</div>
  {:else if $documents.length === 0}
    <div class="empty-state">
      <p>No books indexed yet</p>
      <p class="hint">Upload a PDF to start building a page index</p>
    </div>
  {:else}
    <div class="documents-grid">
      {#each $documents as document (document.id)}
        <DocumentCard
          {document}
          selected={$selectedDocument?.id === document.id}
          on:select={() => handleDocumentSelect(document)}
          on:delete={() => handleDocumentDelete(document.id, document.title)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .document-list {
    width: 100%;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  .header h2 {
    margin: 0;
    font-size: 1.5rem;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .error-message {
    padding: 1rem;
    background: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
    color: var(--accent-color, #c33);
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .loading {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }

  .empty-state {
    text-align: center;
    padding: 3rem;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }

  .empty-state .hint {
    font-size: 0.9rem;
    color: var(--text-tertiary);
    margin-top: 0.5rem;
    transition: color 0.3s ease;
  }

  .documents-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }

  @media screen and (max-width: 768px) {
    .documents-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
