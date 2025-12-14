<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { UrlToMarkdownRequestSchema } from '../../validation/urlToMarkdown.ts'

  interface LinkInfo {
    original: string
    full_url: string
  }

  interface MarkdownResponse {
    markdown: string
    url: string
    internal_links_count: number
    internal_links: LinkInfo[]
  }

  let url = ''
  let markdown = ''
  let loading = false
  let error = ''
  let convertedUrl = ''
  let internalLinks: LinkInfo[] = []
  let internalLinksCount = 0

  // Advanced options
  let showAdvanced = false
  let extractBody = true
  let enablePreprocessing = false
  let removeNavigation = false
  let removeForms = false
  let preprocessingPreset: 'minimal' | 'standard' | 'aggressive' = 'minimal'
  let followLinks = false

  const convertUrlToMarkdown = async () => {
    loading = true
    error = ''
    markdown = ''
    convertedUrl = ''
    internalLinks = []
    internalLinksCount = 0

    try {
      // Validate with Zod
      const validationResult = UrlToMarkdownRequestSchema.safeParse({
        url: url.trim(),
        extract_body: extractBody,
        enable_preprocessing: enablePreprocessing,
        remove_navigation: removeNavigation,
        remove_forms: removeForms,
        preprocessing_preset: enablePreprocessing ? preprocessingPreset : null,
        follow_links: followLinks
      })

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        loading = false
        return
      }

      const requestData = validationResult.data

      const res = await axiosBackendInstance.post<MarkdownResponse | Blob>(
        'url-to-markdown',
        {
          url: requestData.url,
          extract_body: requestData.extract_body,
          enable_preprocessing: requestData.enable_preprocessing,
          remove_navigation: requestData.remove_navigation,
          remove_forms: requestData.remove_forms,
          preprocessing_preset: requestData.preprocessing_preset,
          follow_links: requestData.follow_links
        },
        {
          responseType: followLinks ? 'blob' : 'json'
        }
      )

      if (followLinks && res.data instanceof Blob) {
        // Download the zip file
        const url_blob = window.URL.createObjectURL(res.data)
        const a = document.createElement('a')
        a.href = url_blob
        a.download = `markdown_archive_${Date.now()}.zip`
        document.body.appendChild(a)
        a.click()
        window.URL.revokeObjectURL(url_blob)
        document.body.removeChild(a)

        markdown = 'Zip file downloaded successfully!'
        convertedUrl = url.trim()
        internalLinks = []
        internalLinksCount = 0
      } else {
        const data = res.data as MarkdownResponse
        markdown = data.markdown
        convertedUrl = data.url
        internalLinks = data.internal_links
        internalLinksCount = data.internal_links_count
      }
    } catch (err: any) {
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to convert URL to markdown'
      markdown = ''
      convertedUrl = ''
      internalLinks = []
      internalLinksCount = 0
    } finally {
      loading = false
    }
  }

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      convertUrlToMarkdown()
    }
  }

  const downloadMarkdown = () => {
    if (!markdown) return

    // Create a filename from the URL or use a default
    const urlObj = new URL(convertedUrl || url)
    const hostname = urlObj.hostname.replace(/\./g, '_')
    const filename = `${hostname || 'markdown'}_${Date.now()}.md`

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
</script>

<div class="url-to-markdown">
  <h3>URL to Markdown Converter</h3>
  <div class="input-container">
    <input
      type="text"
      bind:value={url}
      placeholder="Enter a URL to convert to markdown..."
      onkeypress={handleKeyPress}
      disabled={loading}
      class="url-input"
    />
    <button
      onclick={convertUrlToMarkdown}
      disabled={loading || !url.trim()}
      class="convert-button"
    >
      {loading ? 'Converting...' : 'Convert'}
    </button>
  </div>

  <div class="advanced-section">
    <button
      class="advanced-toggle"
      onclick={() => (showAdvanced = !showAdvanced)}
      type="button"
    >
      <span class="toggle-icon">{showAdvanced ? '▼' : '▶'}</span>
      Advanced Options
    </button>

    {#if showAdvanced}
      <div class="advanced-options">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={extractBody} />
          <span>Extract body content only</span>
        </label>

        <label class="checkbox-label">
          <input type="checkbox" bind:checked={enablePreprocessing} />
          <span>Enable preprocessing</span>
        </label>

        <label class="checkbox-label">
          <input type="checkbox" bind:checked={followLinks} />
          <span>Follow internal links (creates zip file)</span>
        </label>

        {#if enablePreprocessing}
          <div class="preprocessing-options">
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={removeNavigation} />
              <span>Remove navigation elements</span>
            </label>

            <label class="checkbox-label">
              <input type="checkbox" bind:checked={removeForms} />
              <span>Remove forms</span>
            </label>

            <div class="select-group">
              <label for="preset-select">Preprocessing Preset:</label>
              <select
                id="preset-select"
                bind:value={preprocessingPreset}
                class="preset-select"
              >
                <option value="minimal">Minimal</option>
                <option value="standard">Standard</option>
                <option value="aggressive">Aggressive</option>
              </select>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if convertedUrl}
    <div class="url-info">
      <strong>Converted URL:</strong>
      <a href={convertedUrl} target="_blank" rel="noopener noreferrer"
        >{convertedUrl}</a
      >
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

  {#if internalLinksCount > 0}
    <div class="links-info">
      <h4>Internal Links Found: {internalLinksCount}</h4>
      <ul class="links-list">
        {#each internalLinks as link}
          <li>
            <a href={link.full_url} target="_blank" rel="noopener noreferrer"
              >{link.original}</a
            >
            <span class="link-url"> → {link.full_url}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .url-to-markdown {
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
  }

  .url-input {
    flex: 1;
    min-width: 200px;
    padding: 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    font-size: 1rem;
    background-color: var(--bg-primary, white);
    color: var(--text-primary, #333);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .url-input:focus {
    outline: none;
    border-color: var(--accent-color, #b12424);
  }

  .url-input:disabled {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
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

  .url-info {
    padding: 0.75rem;
    background-color: #e8f5e9;
    border: 1px solid #c8e6c9;
    border-radius: 4px;
    margin-bottom: 1rem;
  }

  .url-info a {
    color: #2e7d32;
    word-break: break-all;
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

  .links-info {
    margin-top: 1rem;
    padding: 1rem;
    background-color: #e3f2fd;
    border: 1px solid #90caf9;
    border-radius: 4px;
  }

  .links-info h4 {
    margin-top: 0;
    margin-bottom: 0.75rem;
    color: #1976d2;
  }

  .links-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .links-list li {
    padding: 0.5rem 0;
    border-bottom: 1px solid #bbdefb;
  }

  .links-list li:last-child {
    border-bottom: none;
  }

  .links-list a {
    color: #1976d2;
    text-decoration: none;
    font-weight: 500;
  }

  .links-list a:hover {
    text-decoration: underline;
  }

  .link-url {
    color: #666;
    font-size: 0.9rem;
    margin-left: 0.5rem;
  }

  .advanced-section {
    margin-bottom: 1rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    overflow: hidden;
    transition: border-color 0.3s ease;
  }

  .advanced-toggle {
    width: 100%;
    padding: 0.75rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 500;
    color: var(--text-primary, #333);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition:
      background-color 0.2s,
      color 0.3s ease;
  }

  .advanced-toggle:hover {
    background-color: var(--bg-tertiary, #e8e8e8);
  }

  .toggle-icon {
    font-size: 0.8rem;
    color: var(--text-secondary, #666);
    width: 1rem;
    display: inline-block;
    transition: color 0.3s ease;
  }

  .advanced-options {
    padding: 1rem;
    background-color: var(--bg-secondary, #fafafa);
    border-top: 1px solid var(--border-color, #ddd);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    user-select: none;
  }

  .checkbox-label input[type='checkbox'] {
    cursor: pointer;
    width: 1.1rem;
    height: 1.1rem;
  }

  .checkbox-label span {
    color: var(--text-primary, #333);
    font-size: 0.95rem;
    transition: color 0.3s ease;
  }

  .preprocessing-options {
    margin-left: 1.5rem;
    padding-left: 1rem;
    border-left: 2px solid #b12424;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .select-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .select-group label {
    font-size: 0.9rem;
    color: #555;
    font-weight: 500;
  }

  .preset-select {
    padding: 0.5rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    font-size: 0.9rem;
    background-color: var(--bg-primary, white);
    color: var(--text-primary, #333);
    cursor: pointer;
    max-width: 200px;
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .preset-select:focus {
    outline: none;
    border-color: var(--accent-color, #b12424);
  }
</style>
