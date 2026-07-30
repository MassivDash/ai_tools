<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { getBackendUrl } from '@axios/axiosBackendInstance.ts'
  import type { PageIndexUploadResponse } from '@types'
  import { validatePdfFileType } from '@validation/pageindex.ts'
  import Button from '@ui/Button.svelte'
  import IconButton from '@ui/IconButton.svelte'
  import Dropzone from '@ui/Dropzone.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'

  const dispatch = createEventDispatcher()

  interface FileUpload {
    file: File
    id: string | null
    status: 'pending' | 'processing' | 'completed' | 'error'
    message: string
    logs: string[]
  }

  let pendingFiles: File[] = []
  let uploads: FileUpload[] = []
  let uploading = false
  let uploadErrorMessage = ''

  const handleFiles = (newFiles: File[]) => {
    const validFiles = newFiles.filter((f) => validatePdfFileType(f))

    if (validFiles.length !== newFiles.length) {
      uploadErrorMessage =
        'Some files were skipped. Only PDF files are supported.'
    } else {
      uploadErrorMessage = ''
    }

    pendingFiles = [...pendingFiles, ...validFiles]
  }

  const removeFile = (index: number) => {
    pendingFiles = pendingFiles.filter((_, i) => i !== index)
  }

  const allTerminal = () =>
    uploads.length > 0 &&
    uploads.every((u) => u.status === 'completed' || u.status === 'error')

  const updateUploadById = (id: string, patch: Partial<FileUpload>) => {
    uploads = uploads.map((u) => (u.id === id ? { ...u, ...patch } : u))
  }

  const uploadBooks = async () => {
    if (pendingFiles.length === 0) {
      uploadErrorMessage = 'Please select at least one PDF file'
      return
    }

    uploading = true
    uploadErrorMessage = ''

    const filesToUpload = pendingFiles
    uploads = filesToUpload.map((file) => ({
      file,
      id: null,
      status: 'processing',
      message: 'Preparing file...',
      logs: []
    }))
    pendingFiles = []

    try {
      const formData = new FormData()
      filesToUpload.forEach((file) => {
        formData.append('files', file)
      })

      const baseURL = getBackendUrl()
      const uploadUrl = `${baseURL.replace(/\/$/, '')}/pageindex/documents/upload`

      const wsProtocol = baseURL.startsWith('https') ? 'wss' : 'ws'
      const wsBase = baseURL.replace(/^https?:\/\//, '').replace(/\/$/, '')
      const wsUrl = `${wsProtocol}://${wsBase}/pageindex/logs/ws`
      let ws = new WebSocket(wsUrl)

      // Ids assigned by the backend for this batch (filled in once the upload
      // response comes back). Broadcast ws messages for uploads outside this
      // batch (e.g. a concurrent upload in another tab) are ignored.
      let trackedIds: Set<string> = new Set()

      ws.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data)
          const documentId: string | undefined = parsed.document_id
          if (!documentId || !trackedIds.has(documentId)) {
            return
          }

          if (parsed.status === 'log') {
            uploads = uploads.map((u) =>
              u.id === documentId
                ? { ...u, logs: [...u.logs, parsed.message] }
                : u
            )
            return
          }

          if (parsed.status === 'completed') {
            updateUploadById(documentId, {
              status: 'completed',
              message: parsed.message
            })
          } else if (parsed.status === 'error' && parsed.success === false) {
            updateUploadById(documentId, {
              status: 'error',
              message: parsed.message
            })
          } else {
            updateUploadById(documentId, { message: parsed.message })
          }

          if (allTerminal()) {
            dispatch('uploaded')
            ws.close()
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

      const body: PageIndexUploadResponse = await response.json()

      if (!response.ok || !body.success) {
        // Rejected before any indexing started (e.g. local LLM unreachable) -
        // mark every pending upload in this batch as failed with that reason.
        const reason = body.error || body.message || 'Upload failed'
        uploadErrorMessage = reason
        uploads = uploads.map((u) => ({
          ...u,
          status: 'error',
          message: reason
        }))
        ws.close()
        return
      }

      // Match returned document ids back to their upload entry by filename,
      // in submission order (handles duplicate filenames gracefully).
      const remaining = [...uploads]
      for (const doc of body.documents ?? []) {
        const matchIndex = remaining.findIndex(
          (u) => u.id === null && u.file.name === doc.filename
        )
        const index =
          matchIndex !== -1
            ? uploads.findIndex((u) => u === remaining[matchIndex])
            : -1
        if (index !== -1) {
          trackedIds.add(doc.id)
          uploads[index] = { ...uploads[index], id: doc.id }
          remaining.splice(matchIndex, 1)
        }
      }
      uploads = [...uploads]

      dispatch('uploaded')

      // Completion/failure of each individual file is handled asynchronously
      // via the websocket 'completed'/'error' messages above.
    } catch (err: any) {
      console.error('Error uploading books:', err)
      const reason =
        err.response?.data?.error || err.message || 'Failed to upload books'
      uploadErrorMessage = reason
      uploads = uploads.map((u) => ({ ...u, status: 'error', message: reason }))
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

<div class="upload-book">
  <h3>Upload Books</h3>

  <Dropzone
    accept=".pdf"
    multiple={true}
    disabled={uploading}
    buttonText="Browse Files"
    hint="Supported: PDF (multiple files allowed)"
    on:files={(e) => handleFiles(e.detail)}
  />

  {#if uploadErrorMessage && pendingFiles.length === 0 && uploads.length === 0}
    <div class="warning">{uploadErrorMessage}</div>
  {/if}

  {#if pendingFiles.length > 0}
    <div class="files-list">
      <h4>Selected Files ({pendingFiles.length})</h4>
      <div class="files">
        {#each pendingFiles as file, index (file.name + file.size)}
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
              disabled={uploading}
            >
              <MaterialIcon name="close" width="18" height="18" />
            </IconButton>
          </div>
        {/each}
      </div>
      <Button onclick={uploadBooks} disabled={uploading} variant="success">
        {uploading
          ? 'Uploading...'
          : `Upload ${pendingFiles.length} file${pendingFiles.length > 1 ? 's' : ''}`}
      </Button>
    </div>
  {/if}

  {#if uploads.length > 0}
    <div class="uploads-list">
      {#each uploads as upload (upload.file.name + upload.file.size)}
        <div
          class="status"
          class:processing={upload.status === 'processing'}
          class:completed={upload.status === 'completed'}
          class:error={upload.status === 'error'}
        >
          <div class="status-header">
            <span class="status-icon">
              {#if upload.status === 'processing'}
                processing
              {:else if upload.status === 'completed'}
                completed
              {:else}
                error
              {/if}
            </span>
            <span class="status-file-name">{upload.file.name}</span>
            <span class="status-message">{upload.message}</span>
          </div>
          {#if upload.status === 'processing' && upload.logs.length > 0}
            <div class="pageindex-logs-container">
              {#each upload.logs as logMsg}
                <div class="log-line">{logMsg}</div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .upload-book {
    margin-bottom: 2rem;
  }

  .upload-book h3 {
    margin: 0 0 1rem 0;
    color: var(--text-primary, #100f0f);
  }

  .warning {
    padding: 1rem;
    background: rgba(255, 243, 205, 0.3);
    border: 1px solid rgba(255, 193, 7, 0.5);
    border-radius: 8px;
    color: var(--text-secondary);
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

  .uploads-list {
    margin-top: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .status {
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
    flex-wrap: wrap;
  }

  .status-icon {
    font-size: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .status-file-name {
    font-weight: 700;
    font-size: 0.95rem;
    color: var(--text-primary);
  }

  .status-message {
    font-weight: 500;
    font-size: 0.9rem;
    color: var(--text-secondary);
    letter-spacing: -0.01em;
  }

  .pageindex-logs-container {
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

  :global(.dark) .pageindex-logs-container {
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
