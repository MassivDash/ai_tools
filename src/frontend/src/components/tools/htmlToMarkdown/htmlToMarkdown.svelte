<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { HtmlToMarkdownRequestSchema } from '@validation/htmlToMarkdown.ts'

  interface LinkInfo {
    original: string
    full_url: string
    link_text: string
  }

  interface MarkdownResponse {
    markdown: string
    internal_links_count: number
    internal_links: LinkInfo[]
    token_count: number
  }

  let html = ''
  let markdown = ''
  let loading = false
  let error = ''
  let internalLinks: LinkInfo[] = []
  let internalLinksCount = 0
  let tokenCount = 0

  // Advanced options
  let showAdvanced = false
  let extractBody = true
  let enablePreprocessing = false
  let removeNavigation = false
  let removeForms = false
  let preprocessingPreset: 'minimal' | 'standard' | 'aggressive' = 'minimal'
  let countTokens = false

  const convertHtmlToMarkdown = async () => {
    loading = true
    error = ''
    markdown = ''
    internalLinks = []
    internalLinksCount = 0
    tokenCount = 0

    try {
      // Validate with Zod
      const validationResult = HtmlToMarkdownRequestSchema.safeParse({
        html: html.trim(),
        extract_body: extractBody,
        enable_preprocessing: enablePreprocessing,
        remove_navigation: removeNavigation,
        remove_forms: removeForms,
        preprocessing_preset: enablePreprocessing ? preprocessingPreset : null,
        count_tokens: countTokens
      })

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        loading = false
        return
      }

      const requestData = validationResult.data

      const res = await axiosBackendInstance.post<MarkdownResponse>(
        'html-to-markdown',
        {
          html: requestData.html,
          extract_body: requestData.extract_body,
          enable_preprocessing: requestData.enable_preprocessing,
          remove_navigation: requestData.remove_navigation,
          remove_forms: requestData.remove_forms,
          preprocessing_preset: requestData.preprocessing_preset,
          count_tokens: requestData.count_tokens
        }
      )

      const data = res.data
      markdown = data.markdown
      internalLinks = data.internal_links
      internalLinksCount = data.internal_links_count
      tokenCount = data.token_count || 0
    } catch (err: any) {
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to convert HTML to markdown'
      markdown = ''
      internalLinks = []
      internalLinksCount = 0
      tokenCount = 0
    } finally {
      loading = false
    }
  }

  const downloadMarkdown = () => {
    if (!markdown) return

    // Create a filename
    const filename = `html_to_markdown_${Date.now()}.md`

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

  const clearAll = () => {
    html = ''
    markdown = ''
    error = ''
    internalLinks = []
    internalLinksCount = 0
    tokenCount = 0
  }
</script>

<div class="html-to-markdown">
  <h3>HTML to Markdown Converter</h3>
  <div class="input-container">
    <button
      onclick={convertHtmlToMarkdown}
      disabled={loading || !html.trim()}
      class="convert-button"
    >
      {loading ? 'Converting...' : 'Convert'}
    </button>
    <button onclick={clearAll} disabled={loading} class="clear-button">
      Clear
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
          <input type="checkbox" bind:checked={countTokens} />
          <span
            >Count tokens (may slow down conversion for large documents)</span
          >
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

  {#if tokenCount > 0}
    <div class="token-info">
      <strong>Token Count:</strong>
      {tokenCount.toLocaleString()}
    </div>
  {/if}

  <div class="content-container">
    <div class="input-section">
      <div class="section-header">
        <h4>HTML Input</h4>
      </div>
      <textarea
        bind:value={html}
        placeholder="Paste your HTML here..."
        disabled={loading}
        class="html-input"
      ></textarea>
    </div>

    {#if markdown}
      <div class="output-section">
        <div class="section-header">
          <h4>Markdown Output</h4>
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
    {:else}
      <div class="output-section placeholder">
        <div class="section-header">
          <h4>Markdown Output</h4>
        </div>
        <div class="placeholder-content">
          <p>Converted markdown will appear here</p>
        </div>
      </div>
    {/if}
  </div>

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
  .html-to-markdown {
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
    margin-top: 0;
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

  .clear-button {
    padding: 0.75rem 1.5rem;
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--text-primary, #333);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    font-size: 1rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .clear-button:hover:not(:disabled) {
    background-color: var(--bg-tertiary, #e8e8e8);
    border-color: var(--accent-color, #b12424);
  }

  .clear-button:disabled {
    opacity: 0.6;
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

  .token-info {
    padding: 0.75rem;
    background-color: var(--bg-secondary, #e8f5e9);
    border: 1px solid var(--border-color, #c8e6c9);
    border-radius: 4px;
    margin-bottom: 1rem;
    color: var(--text-primary, #333);
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .content-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-top: 1rem;
  }

  .input-section,
  .output-section {
    display: flex;
    flex-direction: column;
    min-height: 500px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .section-header h4 {
    margin: 0;
  }

  .html-input {
    flex: 1;
    padding: 1rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    font-family:
      'Menlo', 'Monaco', 'Lucida Console', 'Liberation Mono',
      'DejaVu Sans Mono', 'Bitstream Vera Sans Mono', 'Courier New', monospace;
    font-size: 0.9rem;
    line-height: 1.5;
    resize: none;
    background-color: var(--bg-primary, white);
    color: var(--text-primary, #333);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .html-input:focus {
    outline: none;
    border-color: var(--accent-color, #b12424);
  }

  .html-input:disabled {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
  }

  .output-section.placeholder {
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    background-color: var(--bg-secondary, #f5f5f5);
  }

  .placeholder-content {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary, #666);
    padding: 2rem;
  }

  .markdown-output {
    flex: 1;
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    padding: 1rem;
    overflow-x: auto;
    overflow-y: auto;
    font-family:
      'Menlo', 'Monaco', 'Lucida Console', 'Liberation Mono',
      'DejaVu Sans Mono', 'Bitstream Vera Sans Mono', 'Courier New', monospace;
    font-size: 0.9rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
    color: var(--text-primary, #333);
    margin: 0;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .markdown-output code {
    font-family: inherit;
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
    color: var(--text-primary, #555);
    font-weight: 500;
    transition: color 0.3s ease;
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

  .links-info {
    margin-top: 1rem;
    padding: 1rem;
    background-color: var(--bg-secondary, #e3f2fd);
    border: 1px solid var(--border-color, #90caf9);
    border-radius: 4px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .links-info h4 {
    margin-top: 0;
    margin-bottom: 0.75rem;
    color: var(--text-primary, #1976d2);
    transition: color 0.3s ease;
  }

  .links-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .links-list li {
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border-color, #bbdefb);
    transition: border-color 0.3s ease;
  }

  .links-list li:last-child {
    border-bottom: none;
  }

  .links-list a {
    color: var(--accent-color, #1976d2);
    text-decoration: none;
    font-weight: 500;
    transition: color 0.3s ease;
  }

  .links-list a:hover {
    text-decoration: underline;
  }

  .link-url {
    color: var(--text-secondary, #666);
    font-size: 0.9rem;
    margin-left: 0.5rem;
    transition: color 0.3s ease;
  }

  @media screen and (max-width: 768px) {
    .content-container {
      grid-template-columns: 1fr;
    }

    .input-section,
    .output-section {
      min-height: 300px;
    }
  }
</style>
