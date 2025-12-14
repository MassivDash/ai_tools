<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { PdfToMarkdownRequestSchema } from '@validation/pdfToMarkdown.ts'

  interface MarkdownResponse {
    markdown: string
    filename: string
  }

  let selectedFile: File | null = null
  let markdown = ''
  let loading = false
  let error = ''
  let convertedFilename = ''

  const handleFileSelect = (event: Event) => {
    const target = event.target as HTMLInputElement
    if (target.files && target.files.length > 0) {
      selectedFile = target.files[0]
      error = ''
      markdown = ''
      convertedFilename = ''
    }
  }

  const convertPdfToMarkdown = async () => {
    if (!selectedFile) {
      error = 'Please select a PDF file'
      return
    }

    loading = true
    error = ''
    markdown = ''
    convertedFilename = ''

    try {
      // Validate with Zod
      const validationResult = PdfToMarkdownRequestSchema.safeParse({
        file: selectedFile
      })

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        loading = false
        return
      }

      // Create FormData for file upload
      const formData = new FormData()
      formData.append('file', selectedFile)

      const res = await axiosBackendInstance.post<MarkdownResponse>(
        'pdf-to-markdown',
        formData,
        {
          headers: {
            'Content-Type': 'multipart/form-data'
          }
        }
      )

      markdown = res.data.markdown
      convertedFilename = res.data.filename || selectedFile.name
    } catch (err: any) {
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to convert PDF to markdown'
      markdown = ''
      convertedFilename = ''
    } finally {
      loading = false
    }
  }

  const downloadMarkdown = () => {
    if (!markdown) return

    // Create a filename from the original PDF filename
    const baseName = convertedFilename.replace(/\.pdf$/i, '')
    const filename = `${baseName}_${Date.now()}.md`

    // Create a blob with the markdown content
    const blob = new Blob([markdown], { type: 'text/markdown' })
    const url_blob = URL.createObjectURL(blob)

    // Create a temporary anchor element and trigger download
    const a = document.createElement('a')
    a.href = url_blob
    a.download = filename
    document.body.appendChild(a)
    a.click()

    // Cleanup
    document.body.removeChild(a)
    URL.revokeObjectURL(url_blob)
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
  }
</script>

<div class="pdf-to-markdown">
  <h3>PDF to Markdown Converter</h3>
  <div class="input-container">
    <label for="pdf-file-input" class="file-input-label">
      <input
        id="pdf-file-input"
        type="file"
        accept=".pdf,application/pdf"
        onchange={handleFileSelect}
        disabled={loading}
        class="file-input"
      />
      <span class="file-input-text">
        {selectedFile
          ? `${selectedFile.name} (${formatFileSize(selectedFile.size)})`
          : 'Choose PDF file...'}
      </span>
    </label>
    <button
      onclick={convertPdfToMarkdown}
      disabled={loading || !selectedFile}
      class="convert-button"
    >
      {loading ? 'Converting...' : 'Convert'}
    </button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if convertedFilename}
    <div class="file-info">
      <strong>Converted File:</strong> {convertedFilename}
    </div>
  {/if}

  {#if markdown}
    <div class="markdown-container">
      <div class="markdown-header">
        <h4>Markdown Output:</h4>
        <button
          onclick={downloadMarkdown}
          class="download-button"
          title="Download markdown file"
          aria-label="Download markdown file"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
          </svg>
          Download
        </button>
      </div>
      <pre class="markdown-output"><code>{markdown}</code></pre>
    </div>
  {/if}
</div>

<style>
  .pdf-to-markdown {
    width: 100%;
    padding: 1rem;
  }

  h3 {
    margin-top: 0;
    margin-bottom: 1rem;
    color: var(--text-primary, #100f0f);
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
  }

  .markdown-container {
    margin-top: 1rem;
  }

  .markdown-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .markdown-header h4 {
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

  .markdown-output {
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

  .markdown-output code {
    font-family: inherit;
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

