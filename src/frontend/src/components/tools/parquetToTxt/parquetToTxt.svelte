<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

  let selectedFiles: File[] = []
  let loading = false
  let error = ''
  let progress = 0
  let currentFile = ''
  let statusMessage = ''

  const handleFileSelect = (event: Event) => {
    const target = event.target as HTMLInputElement
    if (target.files && target.files.length > 0) {
      selectedFiles = Array.from(target.files)
      error = ''
      progress = 0
      currentFile = ''
      statusMessage = ''
    }
  }

  const convertParquetToTxt = async () => {
    if (selectedFiles.length === 0) {
      error = 'Please select at least one parquet file'
      return
    }

    // Validate all files are parquet
    const invalidFiles = selectedFiles.filter(
      (file) => !file.name.toLowerCase().endsWith('.parquet')
    )
    if (invalidFiles.length > 0) {
      error = `Invalid file type. Please select only .parquet files. Found: ${invalidFiles.map((f) => f.name).join(', ')}`
      return
    }

    loading = true
    error = ''
    progress = 0
    currentFile = ''
    statusMessage = 'Preparing files...'

    try {
      // Create FormData for file upload
      const formData = new FormData()
      selectedFiles.forEach((file) => {
        formData.append('files', file)
      })

      // Make request with blob response type for streaming download
      const res = await axiosBackendInstance.post('parquet-to-txt', formData, {
        headers: {
          'Content-Type': 'multipart/form-data'
        },
        responseType: 'blob',
        onDownloadProgress: (progressEvent) => {
          if (progressEvent.total) {
            progress = Math.round(
              (progressEvent.loaded / progressEvent.total) * 100
            )
            statusMessage = `Downloading... ${progress}%`
          } else {
            // If total is unknown, show bytes downloaded
            const mbDownloaded = (progressEvent.loaded / 1024 / 1024).toFixed(
              2
            )
            statusMessage = `Downloading... ${mbDownloaded} MB`
          }
        }
      })

      // Extract filename from Content-Disposition header or use default
      let filename = `imatrix_quantization_data_${Date.now()}.txt`
      const contentDisposition = res.headers['content-disposition']
      if (contentDisposition) {
        const filenameMatch = contentDisposition.match(/filename="?(.+)"?/i)
        if (filenameMatch && filenameMatch[1]) {
          filename = filenameMatch[1]
        }
      }

      // Create blob and trigger download
      const blob = new Blob([res.data], { type: 'text/plain' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = filename
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)

      statusMessage = '✅ Download complete!'
      progress = 100

      // Reset after a short delay
      setTimeout(() => {
        progress = 0
        statusMessage = ''
        loading = false
      }, 2000)
    } catch (err: any) {
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to convert parquet files to text'
      statusMessage = ''
      progress = 0
      loading = false
    }
  }


  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
  }

  const getTotalSize = (): number => {
    return selectedFiles.reduce((sum, file) => sum + file.size, 0)
  }
</script>

<div class="parquet-to-txt">
  <h3>Parquet to TXT Converter</h3>
  <p class="description">
    Combine and convert parquet files to text format for Imatrix Quantization
    Method. Select multiple parquet files to combine them into a single text
    file.
  </p>

  <div class="input-container">
    <label for="parquet-file-input" class="file-input-label">
      <input
        id="parquet-file-input"
        type="file"
        accept=".parquet,application/parquet"
        multiple
        onchange={handleFileSelect}
        disabled={loading}
        class="file-input"
      />
      <span class="file-input-text">
        {selectedFiles.length === 0
          ? 'Choose parquet file(s)...'
          : `${selectedFiles.length} file(s) selected (${formatFileSize(getTotalSize())})`}
      </span>
    </label>
    <button
      onclick={convertParquetToTxt}
      disabled={loading || selectedFiles.length === 0}
      class="convert-button"
    >
      {loading ? 'Converting...' : 'Convert'}
    </button>
  </div>

  {#if selectedFiles.length > 0}
    <div class="files-list">
      <h4>Selected Files:</h4>
      <ul>
        {#each selectedFiles as file}
          <li>
            {file.name} ({formatFileSize(file.size)})
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if loading || statusMessage}
    <div class="progress-container">
      {#if statusMessage}
        <div class="status-message">{statusMessage}</div>
      {/if}
      {#if progress > 0}
        <div class="progress-bar-container">
          <div class="progress-bar" style="width: {progress}%"></div>
        </div>
        <div class="progress-text">{progress}%</div>
      {/if}
      {#if currentFile}
        <div class="current-file">Processing: {currentFile}</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .parquet-to-txt {
    width: 100%;
    padding: 1rem;
  }

  h3 {
    margin-top: 0;
    margin-bottom: 0.5rem;
    color: var(--text-primary, #100f0f);
    transition: color 0.3s ease;
  }

  .description {
    margin-bottom: 1.5rem;
    color: var(--text-secondary, #666);
    font-size: 0.95rem;
    line-height: 1.5;
    transition: color 0.3s ease;
  }

  h4 {
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .input-container {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .file-input-label {
    flex: 1;
    min-width: 200px;
    position: relative;
    display: inline-block;
    cursor: pointer;
  }

  .file-input {
    position: absolute;
    width: 0.1px;
    height: 0.1px;
    opacity: 0;
    overflow: hidden;
    z-index: -1;
  }

  .file-input-text {
    display: block;
    padding: 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    font-size: 1rem;
    background-color: var(--bg-primary, white);
    color: var(--text-primary, #333);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .file-input-label:hover .file-input-text {
    border-color: var(--accent-color, #b12424);
    background-color: var(--bg-secondary, #f9f9f9);
  }

  .file-input:disabled + .file-input-text {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
    opacity: 0.6;
  }

  .convert-button {
    padding: 0.75rem 1.5rem;
    background-color: var(--accent-color, #b12424);
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 1rem;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .convert-button:hover:not(:disabled) {
    background-color: var(--accent-hover, #8a1c1c);
  }

  .convert-button:disabled {
    background-color: var(--text-tertiary, #ccc);
    cursor: not-allowed;
  }

  .files-list {
    margin-bottom: 1rem;
    padding: 0.75rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .files-list h4 {
    margin-top: 0;
    margin-bottom: 0.5rem;
    color: var(--text-primary, #333);
  }

  .files-list ul {
    margin: 0;
    padding-left: 1.5rem;
    color: var(--text-primary, #333);
  }

  .files-list li {
    margin-bottom: 0.25rem;
  }

  .error {
    padding: 0.75rem;
    background-color: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 4px;
    color: var(--accent-color, #c33);
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .file-info {
    padding: 0.75rem;
    background-color: var(--bg-secondary, #e8f5e9);
    border: 1px solid var(--border-color, #c8e6c9);
    border-radius: 4px;
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
    color: var(--text-primary, #333);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .filenames-list {
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-color, #c8e6c9);
  }

  .filenames-list ul {
    margin: 0.5rem 0 0 0;
    padding-left: 1.5rem;
  }

  .filenames-list li {
    margin-bottom: 0.25rem;
  }

  .text-container {
    margin-top: 1rem;
  }

  .text-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .text-header h4 {
    margin: 0;
  }

  .download-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background-color: #1976d2;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .download-button:hover {
    background-color: #1565c0;
  }

  .download-button:active {
    background-color: #0d47a1;
  }

  .download-button svg {
    flex-shrink: 0;
  }

  .text-output {
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    padding: 1rem;
    overflow-x: auto;
    max-height: 600px;
    overflow-y: auto;
    font-family:
      'Menlo', 'Monaco', 'Lucida Console', 'Liberation Mono',
      'DejaVu Sans Mono', 'Bitstream Vera Sans Mono', 'Courier New', monospace;
    font-size: 0.9rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
    color: var(--text-primary, #333);
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .progress-container {
    margin: 1rem 0;
    padding: 1rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .status-message {
    margin-bottom: 0.75rem;
    font-weight: 500;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .progress-bar-container {
    width: 100%;
    height: 24px;
    background-color: var(--bg-tertiary, #e0e0e0);
    border-radius: 12px;
    overflow: hidden;
    margin-bottom: 0.5rem;
    transition: background-color 0.3s ease;
  }

  .progress-bar {
    height: 100%;
    background: linear-gradient(
      90deg,
      var(--accent-color, #b12424) 0%,
      var(--accent-hover, #8a1c1c) 100%
    );
    transition: width 0.3s ease;
    border-radius: 12px;
  }

  .progress-text {
    text-align: center;
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
    margin-bottom: 0.5rem;
    transition: color 0.3s ease;
  }

  .current-file {
    font-size: 0.85rem;
    color: var(--text-secondary, #666);
    font-style: italic;
    transition: color 0.3s ease;
  }

  @media screen and (max-width: 768px) {
    .input-container {
      flex-direction: column;
    }

    .file-input-label {
      width: 100%;
    }
  }
</style>

