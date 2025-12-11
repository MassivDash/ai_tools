<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

  interface MarkdownResponse {
    markdown: string
    url: string
  }

  let url = ''
  let markdown = ''
  let loading = false
  let error = ''
  let convertedUrl = ''

  const convertUrlToMarkdown = async () => {
    if (!url.trim()) {
      error = 'Please enter a valid URL'
      return
    }

    loading = true
    error = ''
    markdown = ''
    convertedUrl = ''

    try {
      const res = await axiosBackendInstance.post<MarkdownResponse>(
        'url-to-markdown',
        { url: url.trim() }
      )
      markdown = res.data.markdown
      convertedUrl = res.data.url
    } catch (err: any) {
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to convert URL to markdown'
      markdown = ''
      convertedUrl = ''
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

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if convertedUrl}
    <div class="url-info">
      <strong>Converted URL:</strong> <a href={convertedUrl} target="_blank" rel="noopener noreferrer">{convertedUrl}</a>
    </div>
  {/if}

  {#if markdown}
    <div class="markdown-container">
      <h4>Markdown Output:</h4>
      <pre class="markdown-output"><code>{markdown}</code></pre>
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
    color: #100f0f;
  }

  h4 {
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    color: #333;
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
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 1rem;
  }

  .url-input:focus {
    outline: none;
    border-color: #b12424;
  }

  .url-input:disabled {
    background-color: #f5f5f5;
    cursor: not-allowed;
  }

  .convert-button {
    padding: 0.75rem 1.5rem;
    background-color: #b12424;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 1rem;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .convert-button:hover:not(:disabled) {
    background-color: #8a1c1c;
  }

  .convert-button:disabled {
    background-color: #ccc;
    cursor: not-allowed;
  }

  .error {
    padding: 0.75rem;
    background-color: #fee;
    border: 1px solid #fcc;
    border-radius: 4px;
    color: #c33;
    margin-bottom: 1rem;
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

  .markdown-output {
    background-color: #f5f5f5;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 1rem;
    overflow-x: auto;
    max-height: 600px;
    overflow-y: auto;
    font-family: 'Menlo', 'Monaco', 'Lucida Console', 'Liberation Mono',
      'DejaVu Sans Mono', 'Bitstream Vera Sans Mono', 'Courier New', monospace;
    font-size: 0.9rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  .markdown-output code {
    font-family: inherit;
  }
</style>

