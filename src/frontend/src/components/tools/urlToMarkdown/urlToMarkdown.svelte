<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { UrlToMarkdownRequestSchema } from '@validation/urlToMarkdown.ts'
  import Button from '../../ui/Button.svelte'
  import Input from '../../ui/Input.svelte'
  import Select from '../../ui/Select.svelte'

  interface LinkInfo {
    original: string
    full_url: string
  }

  interface MarkdownResponse {
    markdown: string
    url: string
    internal_links_count: number
    internal_links: LinkInfo[]
    token_count: number
  }

  let url = ''
  let markdown = ''
  let loading = false
  let error = ''
  let convertedUrl = ''
  let internalLinks: LinkInfo[] = []
  let internalLinksCount = 0
  let tokenCount = 0

  // Advanced options
  let showAdvanced = false
  let extractBody = false
  let enablePreprocessing = false
  let removeNavigation = false
  let removeForms = false
  let preprocessingPreset: 'minimal' | 'standard' | 'aggressive' = 'minimal'
  let followLinks = false
  let countTokens = false

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
        follow_links: followLinks,
        count_tokens: countTokens
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
          follow_links: requestData.follow_links,
          count_tokens: requestData.count_tokens
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
        tokenCount = data.token_count || 0
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
      tokenCount = 0
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
    <Input
      type="text"
      bind:value={url}
      placeholder="Enter a URL to convert to markdown..."
      onkeypress={handleKeyPress}
      disabled={loading}
    />
    <Button
      onclick={convertUrlToMarkdown}
      disabled={loading || !url.trim()}
      variant="primary"
    >
      {loading ? 'Converting...' : 'Convert'}
    </Button>
  </div>

  <div class="advanced-section">
    <Button
      class="advanced-toggle"
      onclick={() => (showAdvanced = !showAdvanced)}
      variant="ghost"
    >
      <span class="toggle-icon">{showAdvanced ? '▼' : '▶'}</span>
      Advanced Options
    </Button>

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

            <Select
              label="Preprocessing Preset:"
              id="preset-select"
              bind:value={preprocessingPreset}
              options={[
                { value: 'minimal', label: 'Minimal' },
                { value: 'standard', label: 'Standard' },
                { value: 'aggressive', label: 'Aggressive' }
              ]}
              class="preset-select"
            />
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
      <div>
        <strong>Converted URL:</strong>
        <a href={convertedUrl} target="_blank" rel="noopener noreferrer"
          >{convertedUrl}</a
        >
      </div>
      {#if tokenCount > 0}
        <div class="token-info">
          <strong>Token Count:</strong>
          {tokenCount.toLocaleString()}
        </div>
      {/if}
    </div>
  {/if}

  {#if markdown}
    <div class="markdown-container">
      <div class="markdown-header">
        <h4>Markdown Output:</h4>
        <Button
          onclick={downloadMarkdown}
          class="download-button"
          variant="primary"
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
        </Button>
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

  .input-container :global(.input-wrapper) {
    flex: 1;
    min-width: 200px;
    margin-bottom: 0 !important;
  }

  .error {
    padding: 0.75rem;
    background-color: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
    color: var(--accent-color, #c33);
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .url-info {
    padding: 0.75rem;
    background-color: var(--bg-secondary, #e8f5e9);
    border: 1px solid var(--border-color, #c8e6c9);
    border-radius: 8px;
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .url-info a {
    color: var(--accent-color, #2e7d32);
    word-break: break-all;
    transition: color 0.3s ease;
  }

  .token-info {
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-color, #c8e6c9);
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

  .markdown-output {
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
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
    background-color: var(--bg-secondary, #e3f2fd);
    border: 1px solid var(--border-color, #90caf9);
    border-radius: 8px;
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

  .advanced-section {
    margin-bottom: 1rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    overflow: hidden;
    transition: border-color 0.3s ease;
  }

  :global(.advanced-toggle) {
    width: 100%;
    padding: 0.75rem;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition:
      background-color 0.2s,
      color 0.3s ease;
    justify-content: flex-start;
  }

  :global(.advanced-toggle:hover) {
    background-color: var(--bg-tertiary, #e8e8e8);
  }

  .toggle-icon {
    font-size: 0.8rem;
    color: var(--text-secondary, #666);
    width: 1rem;
    display: inline-block;
    transition: color 0.3s ease;
  }

  :global(.preset-select) {
    max-width: 200px;
  }
</style>
