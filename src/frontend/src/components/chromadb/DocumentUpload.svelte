<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ChromaDBResponse, ProcessingStatus } from '../../types/chromadb.ts'
  import Button from '../ui/Button.svelte'
  import Dropzone from '../ui/Dropzone.svelte'
  import XIcon from '../ui/icons/XIcon.svelte'

  export let selectedCollection: string | null = null

  const dispatch = createEventDispatcher()

  let files: File[] = []
  let uploading = false
  let progress = 0
  let error = ''
  let status: ProcessingStatus | null = null

  const handleFiles = (newFiles: File[]) => {
    // Filter for supported file types
    const supportedTypes = ['application/pdf', 'text/markdown', 'text/plain', 'text/mdx']
    const validFiles = newFiles.filter(
      (file) =>
        supportedTypes.includes(file.type) ||
        file.name.endsWith('.pdf') ||
        file.name.endsWith('.md') ||
        file.name.endsWith('.mdx') ||
        file.name.endsWith('.txt')
    )

    if (validFiles.length !== newFiles.length) {
      error = 'Some files were skipped. Only PDF, Markdown, and text files are supported.'
    } else {
      error = ''
    }

    files = [...files, ...validFiles]
  }

  const removeFile = (index: number) => {
    files = files.filter((_, i) => i !== index)
  }

  const uploadDocuments = async () => {
    if (!selectedCollection) {
      error = 'Please select a collection first'
      return
    }

    if (files.length === 0) {
      error = 'Please select at least one file'
      return
    }

    uploading = true
    error = ''
    progress = 0
    status = {
      status: 'processing',
      progress: 0,
      message: 'Preparing files...',
      processed_files: 0,
      total_files: files.length
    }

    try {
      console.log('📤 Uploading documents to collection:', selectedCollection)

      const formData = new FormData()
      formData.append('collection', selectedCollection)
      files.forEach((file) => {
        formData.append('files', file)
      })

      const response = await axiosBackendInstance.post<ChromaDBResponse<void>>(
        'chromadb/documents/upload',
        formData,
        {
          headers: {
            'Content-Type': 'multipart/form-data'
          },
          onUploadProgress: (progressEvent) => {
            if (progressEvent.total) {
              progress = Math.round((progressEvent.loaded * 100) / progressEvent.total)
              if (status) {
                status.progress = progress
                status.message = `Uploading... ${progress}%`
              }
            }
          }
        }
      )

      if (response.data.success) {
        console.log('✅ Documents uploaded successfully')
        status = {
          status: 'completed',
          progress: 100,
          message: response.data.message || 'Documents uploaded successfully',
          processed_files: files.length,
          total_files: files.length
        }
        dispatch('uploaded', { collection: selectedCollection, files: files.length })
        // Clear files after successful upload
        files = []
      } else {
        error = response.data.error || 'Failed to upload documents'
        status = {
          status: 'error',
          progress: 0,
          message: error,
          processed_files: 0,
          total_files: files.length
        }
      }
    } catch (err: any) {
      console.error('❌ Error uploading documents:', err)
      error = err.response?.data?.error || err.message || 'Failed to upload documents'
      status = {
        status: 'error',
        progress: 0,
        message: error,
        processed_files: 0,
        total_files: files.length
      }
    } finally {
      uploading = false
    }
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
  }
</script>

<div class="document-upload">
  <h3>Upload Documents</h3>

  {#if !selectedCollection}
    <div class="warning">
      ⚠️ Please select a collection first to upload documents
    </div>
  {:else}
    <Dropzone
      accept=".pdf,.md,.mdx,.txt"
      multiple={true}
      disabled={uploading}
      buttonText="Browse Files"
      hint="Supported: PDF, Markdown (.md, .mdx), Text (.txt)"
      on:files={(e) => handleFiles(e.detail)}
    />

    {#if files.length > 0}
      <div class="files-list">
        <h4>Selected Files ({files.length})</h4>
        <div class="files">
          {#each files as file, index (file.name + file.size)}
            <div class="file-item">
              <div class="file-info">
                <span class="file-name">{file.name}</span>
                <span class="file-size">{formatFileSize(file.size)}</span>
              </div>
              <button class="remove-file-btn" onclick={() => removeFile(index)} type="button" title="Remove file">
                <XIcon width="18" height="18" />
              </button>
            </div>
          {/each}
        </div>
        <Button onclick={uploadDocuments} disabled={uploading || !selectedCollection} variant="success">
          {uploading ? 'Uploading...' : `Upload ${files.length} file${files.length > 1 ? 's' : ''}`}
        </Button>
      </div>
    {/if}

    {#if error}
      <div class="error-message">❌ {error}</div>
    {/if}

    {#if status}
      <div class="status" class:processing={status.status === 'processing'} class:completed={status.status === 'completed'} class:error={status.status === 'error'}>
        <div class="status-header">
          <span class="status-icon">
            {#if status.status === 'processing'}⏳
            {:else if status.status === 'completed'}✅
            {:else}❌
            {/if}
          </span>
          <span class="status-message">{status.message}</span>
        </div>
        {#if status.status === 'processing'}
          <div class="progress-bar">
            <div class="progress-fill" style="width: {status.progress}%"></div>
          </div>
          <div class="progress-text">
            {status.processed_files} / {status.total_files} files processed
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .document-upload {
    margin-bottom: 2rem;
  }

  .document-upload h3 {
    margin: 0 0 1rem 0;
    color: var(--text-primary, #100f0f);
  }

  .warning {
    padding: 1rem;
    background: rgba(255, 243, 205, 0.3);
    border: 1px solid rgba(255, 193, 7, 0.5);
    border-radius: 4px;
    color: var(--text-secondary);
    transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease;
  }

  .files-list {
    margin-top: 1.5rem;
    padding: 1rem;
    background: var(--bg-primary, white);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
  }

  .files-list h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary, #100f0f);
  }

  .files {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .file-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background: var(--bg-secondary, #f5f5f5);
    border-radius: 4px;
  }

  .file-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .file-name {
    font-weight: 500;
    color: var(--text-primary, #100f0f);
  }

  .file-size {
    font-size: 0.85rem;
    color: var(--text-secondary, #666);
  }

  .remove-file-btn {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    cursor: pointer;
    padding: 0.4rem;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.8;
    transition: opacity 0.2s, background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease;
    color: var(--text-primary);
  }

  .remove-file-btn:hover {
    opacity: 1;
    background: var(--bg-tertiary);
    border-color: var(--border-color-hover);
    color: var(--accent-color, #c33);
  }

  .error-message {
    padding: 1rem;
    background: #fee;
    border: 1px solid #fcc;
    border-radius: 4px;
    color: #c33;
    margin-top: 1rem;
  }

  .status {
    margin-top: 1rem;
    padding: 1rem;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .status.processing {
    background: rgba(33, 150, 243, 0.1);
    border-color: rgba(33, 150, 243, 0.3);
  }

  .status.completed {
    background: rgba(76, 175, 80, 0.1);
    border-color: rgba(76, 175, 80, 0.3);
  }

  .status.error {
    background: rgba(244, 67, 54, 0.1);
    border-color: rgba(244, 67, 54, 0.3);
  }

  .status-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .status-icon {
    font-size: 1.2rem;
  }

  .status-message {
    font-weight: 500;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .progress-bar {
    width: 100%;
    height: 8px;
    background: var(--bg-secondary);
    border-radius: 4px;
    overflow: hidden;
    margin: 0.5rem 0;
    transition: background-color 0.3s ease;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent-color, #4a90e2);
    transition: width 0.3s ease, background-color 0.3s ease;
  }

  .progress-text {
    font-size: 0.9rem;
    color: var(--text-secondary);
    text-align: center;
    transition: color 0.3s ease;
  }
</style>

