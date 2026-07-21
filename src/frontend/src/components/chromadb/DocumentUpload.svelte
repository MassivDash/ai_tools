<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { getBackendUrl } from '@axios/axiosBackendInstance.ts'
  import type { ProcessingStatus } from '../../types'
  import {
    DocumentUploadSchema,
    validateFileType
  } from '@validation/chromadb.ts'
  import Button from '@ui/Button.svelte'
  import IconButton from '@ui/IconButton.svelte'
  import Dropzone from '@ui/Dropzone.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'

  export let selectedCollection: string | null = null

  const dispatch = createEventDispatcher()

  let files: File[] = []
  let uploading = false

  let _error = ''
  let status: ProcessingStatus | null = null
  let logs: string[] = []

  // Reset upload area when collection changes
  $: if (selectedCollection) {
    files = []
    uploading = false
    _error = ''
    status = null
    logs = []
  }

  const handleFiles = (newFiles: File[]) => {
    // Filter for supported file types using Zod validation
    const validFiles = newFiles.filter((file) => {
      try {
        return validateFileType(file)
      } catch {
        return false
      }
    })

    if (validFiles.length !== newFiles.length) {
      _error = ''
      status = {
        status: 'error',
        progress: 0,
        message:
          'Some files were skipped. Only PDF, Markdown, and text files are supported.',
        processed_files: 0,
        total_files: 0
      }
    } else {
      _error = ''
      status = null
    }

    files = [...files, ...validFiles]
  }

  const removeFile = (index: number) => {
    files = files.filter((_, i) => i !== index)
  }

  const uploadDocuments = async () => {
    uploading = true
    _error = ''
    logs = []

    try {
      // Validate collection name with Zod
      const collectionValidation = DocumentUploadSchema.safeParse({
        collection: selectedCollection
      })

      if (!collectionValidation.success) {
        const firstError = collectionValidation.error.issues[0]
        _error = ''
        status = {
          status: 'error',
          progress: 0,
          message: firstError.message,
          processed_files: 0,
          total_files: 0
        }
        uploading = false
        return
      }

      // Validate files
      if (files.length === 0) {
        _error = ''
        status = {
          status: 'error',
          progress: 0,
          message: 'Please select at least one file',
          processed_files: 0,
          total_files: 0
        }
        uploading = false
        return
      }

      // Validate each file type
      const invalidFiles = files.filter((file) => !validateFileType(file))
      if (invalidFiles.length > 0) {
        _error = ''
        status = {
          status: 'error',
          progress: 0,
          message:
            'Some files have invalid types. Only PDF, Markdown, and text files are supported.',
          processed_files: 0,
          total_files: 0
        }
        uploading = false
        return
      }

      status = {
        status: 'processing',
        progress: 0,
        message: 'Preparing files...',
        processed_files: 0,
        total_files: files.length
      }

      const formData = new FormData()
      formData.append('collection', selectedCollection!)
      files.forEach((file) => {
        formData.append('files', file)
      })

      const baseURL = getBackendUrl()
      const uploadUrl = `${baseURL.replace(/\/$/, '')}/chromadb/documents/upload`

      const wsProtocol = baseURL.startsWith('https') ? 'wss' : 'ws'
      const wsBase = baseURL.replace(/^https?:\/\//, '').replace(/\/$/, '')
      const wsUrl = `${wsProtocol}://${wsBase}/chromadb/logs/ws`
      let ws = new WebSocket(wsUrl)

      ws.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data)
          if (parsed.status === 'log') {
            logs = [...logs, parsed.message]
            setTimeout(() => {
              const container = document.querySelector('.logs-container')
              if (container) {
                container.scrollTop = container.scrollHeight
              }
            }, 0)
          } else if (status) {
            status.message = parsed.message
            if (parsed.processed_files !== undefined) {
              status.processed_files = parsed.processed_files
            }
            if (parsed.total_files !== undefined) {
              status.total_files = parsed.total_files
            }
            if (parsed.status === 'processing') {
              status.progress = 100
            }

            if (parsed.status === 'completed') {
              status.status = 'completed'
              status.progress = 100
              status.processed_files = files.length
              status.total_files = files.length
              dispatch('uploaded', {
                collection: selectedCollection,
                files: files.length
              })
              files = []
              ws.close()
            } else if (parsed.status === 'error' && parsed.success === false) {
              _error = parsed.message
              status.status = 'error'
              status.progress = 0
              ws.close()
            }
            status = { ...status }
          }
        } catch (e) {
          console.error('Error parsing WS json', e)
        }
      }

      const response = await new Promise<globalThis.Response>(
        (resolve, reject) => {
          ws.onopen = async () => {
            try {
              const res = await window.fetch(uploadUrl, {
                method: 'POST',
                body: formData
              })
              resolve(res)
            } catch (err) {
              reject(err)
            }
          }
          ws.onerror = () => {
            reject(new Error('WebSocket connection failed'))
          }
        }
      )

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`)
      }

      // We no longer await successStatus here because the completion is handled asynchronously by the WebSocket message 'completed'
    } catch (err: any) {
      console.error('Error uploading documents:', err)
      _error = ''
      status = {
        status: 'error',
        progress: 0,
        message:
          err.response?.data?.error ||
          err.message ||
          'Failed to upload documents',
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
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
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
              <IconButton
                variant="ghost"
                class="remove-file-btn"
                onclick={() => removeFile(index)}
                title="Remove file"
                iconSize={18}
              >
                <MaterialIcon name="close" width="18" height="18" />
              </IconButton>
            </div>
          {/each}
        </div>
        <Button
          onclick={uploadDocuments}
          disabled={uploading || !selectedCollection}
          variant="success"
        >
          {uploading
            ? 'Uploading...'
            : `Upload ${files.length} file${files.length > 1 ? 's' : ''}`}
        </Button>
      </div>
    {/if}

    {#if status}
      <div
        class="status"
        class:processing={status.status === 'processing'}
        class:completed={status.status === 'completed'}
        class:error={status.status === 'error'}
      >
        <div class="status-header">
          <span class="status-icon">
            {#if status.status === 'processing'}
              processing
            {:else if status.status === 'completed'}
              completed
            {:else}
              error
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
          {#if logs.length > 0}
            <div class="logs-container">
              {#each logs as logMsg}
                <div class="log-line">{logMsg}</div>
              {/each}
            </div>
          {/if}
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
    border-radius: 8px;
    color: var(--text-secondary);
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
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
    border-radius: 8px;
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

  .status {
    margin-top: 1rem;
    padding: 1.25rem;
    border-radius: 12px;
    border: 1px solid var(--border-color, #e0e0e0);
    background: var(--bg-primary, #ffffff);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  /* Make status states pop with subtle backgrounds and border colors */
  .status.processing {
    background: linear-gradient(
      145deg,
      rgba(33, 150, 243, 0.05),
      rgba(33, 150, 243, 0.02)
    );
    border-color: rgba(33, 150, 243, 0.3);
  }

  .status.completed {
    background: linear-gradient(
      145deg,
      rgba(76, 175, 80, 0.05),
      rgba(76, 175, 80, 0.02)
    );
    border-color: rgba(76, 175, 80, 0.3);
  }

  .status.error {
    background: linear-gradient(
      145deg,
      rgba(244, 67, 54, 0.05),
      rgba(244, 67, 54, 0.02)
    );
    border-color: rgba(244, 67, 54, 0.3);
  }

  .status-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .status-icon {
    font-size: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .status-message {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }

  .progress-bar {
    width: 100%;
    height: 6px;
    background: var(--bg-secondary, #f0f0f0);
    border-radius: 999px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent-color, #4a90e2);
    border-radius: 999px;
    transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .progress-text {
    text-align: right;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .logs-container {
    margin-top: 0.5rem;
    padding: 1rem;
    background-color: var(--bg-secondary, #f8f9fa);
    border: 1px solid var(--border-color, #e0e0e0);
    color: var(--text-secondary, #666);
    border-radius: 8px;
    font-family:
      'Fira Code', 'JetBrains Mono', 'Courier New', Courier, monospace;
    font-size: 0.8125rem;
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.02);
  }

  /* Target dark mode contexts specifically if the global theme uses dark by default */
  :global(.dark) .logs-container {
    background-color: #121212;
    border-color: #333;
    color: #a0a0a0;
  }

  .log-line {
    word-break: break-word;
    white-space: pre-wrap;
    line-height: 1.4;
  }
</style>
