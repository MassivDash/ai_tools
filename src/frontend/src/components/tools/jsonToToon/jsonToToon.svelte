<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { JsonToToonRequestSchema } from '@validation/jsonToToon.ts'
  import CheckboxWithHelp from '../../ui/CheckboxWithHelp.svelte'

  interface ToonResponse {
    toon: string
    json_tokens: number
    toon_tokens: number
    token_savings: number
  }

  let json = ''
  let toon = ''
  let loading = false
  let error = ''
  let jsonTokens = 0
  let toonTokens = 0
  let tokenSavings = 0
  let selectedFile: File | null = null
  let inputMode: 'paste' | 'file' = 'paste'

  // Advanced options
  let showAdvanced = false
  let countTokens = false

  const validateJson = (
    jsonString: string
  ): { valid: boolean; error?: string } => {
    if (!jsonString.trim()) {
      return { valid: false, error: 'JSON content cannot be empty' }
    }

    try {
      JSON.parse(jsonString)
      return { valid: true }
    } catch (e: any) {
      return { valid: false, error: `Invalid JSON: ${e.message}` }
    }
  }

  const handleFileSelect = async (event: Event) => {
    const target = event.target as HTMLInputElement
    if (target.files && target.files.length > 0) {
      const file = target.files[0]
      selectedFile = file
      error = ''
      json = ''
      toon = ''

      // Read file content
      const reader = new FileReader()
      reader.onload = (e) => {
        const content = e.target?.result as string
        const validation = validateJson(content)
        if (validation.valid) {
          json = content
        } else {
          error = validation.error || 'Invalid JSON file'
          selectedFile = null
        }
      }
      reader.onerror = () => {
        error = 'Failed to read file'
        selectedFile = null
      }
      reader.readAsText(file)
    }
  }

  const convertJsonToToon = async () => {
    // Validate JSON first
    const validation = validateJson(json)
    if (!validation.valid) {
      error = validation.error || 'Invalid JSON'
      return
    }

    loading = true
    error = ''
    toon = ''
    jsonTokens = 0
    toonTokens = 0
    tokenSavings = 0

    try {
      // Validate with Zod
      const validationResult = JsonToToonRequestSchema.safeParse({
        json: json.trim(),
        count_tokens: countTokens
      })

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        loading = false
        return
      }

      const requestData = validationResult.data

      let res
      if (inputMode === 'file' && selectedFile) {
        // Upload file using FormData
        const formData = new FormData()
        formData.append('file', selectedFile)
        formData.append('count_tokens', countTokens.toString())

        res = await axiosBackendInstance.post<ToonResponse>(
          'json-to-toon',
          formData,
          {
            headers: {
              'Content-Type': 'multipart/form-data'
            }
          }
        )
      } else {
        // Send JSON as text
        res = await axiosBackendInstance.post<ToonResponse>('json-to-toon', {
          json: requestData.json,
          count_tokens: requestData.count_tokens
        })
      }

      const data = res.data
      toon = data.toon
      jsonTokens = data.json_tokens || 0
      toonTokens = data.toon_tokens || 0
      tokenSavings = data.token_savings || 0
    } catch (err: any) {
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to convert JSON to TOON'
      toon = ''
      jsonTokens = 0
      toonTokens = 0
      tokenSavings = 0
    } finally {
      loading = false
    }
  }

  const downloadToon = () => {
    if (!toon) return

    const filename = `json_to_toon_${Date.now()}.toon`

    const blob = new Blob([toon], { type: 'text/plain' })
    const url_blob = URL.createObjectURL(blob)

    const a = document.createElement('a')
    a.href = url_blob
    a.download = filename
    document.body.appendChild(a)
    a.click()

    document.body.removeChild(a)
    URL.revokeObjectURL(url_blob)
  }

  const clearAll = () => {
    json = ''
    toon = ''
    error = ''
    selectedFile = null
    jsonTokens = 0
    toonTokens = 0
    tokenSavings = 0
  }

  const formatJson = () => {
    const validation = validateJson(json)
    if (!validation.valid) {
      error = validation.error || 'Invalid JSON'
      return
    }

    try {
      const parsed = JSON.parse(json)
      json = JSON.stringify(parsed, null, 2)
      error = ''
    } catch (e: any) {
      error = `Failed to format JSON: ${e.message}`
    }
  }
</script>

<div class="json-to-toon">
  <h3>JSON to TOON Converter</h3>

  <div class="input-mode-selector">
    <button
      class="mode-button"
      class:active={inputMode === 'paste'}
      onclick={() => {
        inputMode = 'paste'
        selectedFile = null
        json = ''
      }}
      type="button"
    >
      📝 Paste JSON
    </button>
    <button
      class="mode-button"
      class:active={inputMode === 'file'}
      onclick={() => {
        inputMode = 'file'
        json = ''
      }}
      type="button"
    >
      📁 Upload File
    </button>
  </div>

  <div class="input-container">
    {#if inputMode === 'file'}
      <label for="json-file-input" class="file-input-label">
        <input
          id="json-file-input"
          type="file"
          accept=".json,application/json"
          onchange={handleFileSelect}
          disabled={loading}
          class="file-input"
        />
        <span class="file-input-text">
          {selectedFile
            ? `${selectedFile.name} (${(selectedFile.size / 1024).toFixed(2)} KB)`
            : 'Choose JSON file...'}
        </span>
      </label>
    {/if}

    <div class="button-group">
      <button
        onclick={convertJsonToToon}
        disabled={loading || (!json.trim() && !selectedFile)}
        class="convert-button"
      >
        {loading ? 'Converting...' : 'Convert'}
      </button>
      <button onclick={clearAll} disabled={loading} class="clear-button">
        Clear
      </button>
      {#if inputMode === 'paste' && json.trim()}
        <button onclick={formatJson} disabled={loading} class="format-button">
          Format JSON
        </button>
      {/if}
    </div>
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
        <CheckboxWithHelp
          bind:checked={countTokens}
          label="Count tokens"
          helpText="may slow down conversion for large documents"
        />
      </div>
    {/if}
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if countTokens && (jsonTokens > 0 || toonTokens > 0)}
    <div class="token-info">
      <div class="token-stats">
        <div class="token-stat">
          <strong>JSON Tokens:</strong>
          {jsonTokens.toLocaleString()}
        </div>
        <div class="token-stat">
          <strong>TOON Tokens:</strong>
          {toonTokens.toLocaleString()}
        </div>
        {#if tokenSavings > 0}
          <div class="token-stat savings">
            <strong>Savings:</strong>
            {tokenSavings.toFixed(1)}%
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <div class="content-container">
    <div class="input-section">
      <div class="section-header">
        <h4>JSON Input</h4>
      </div>
      {#if inputMode === 'paste'}
        <textarea
          bind:value={json}
          placeholder="Paste your JSON here..."
          disabled={loading}
          class="json-input"
        ></textarea>
      {:else}
        <div class="file-preview">
          {#if selectedFile && json}
            <pre class="json-preview"><code>{json}</code></pre>
          {:else}
            <div class="placeholder-content">
              <p>Select a JSON file to preview</p>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    {#if toon}
      <div class="output-section">
        <div class="section-header">
          <h4>TOON Output</h4>
          <button
            onclick={downloadToon}
            class="download-button"
            title="Download TOON file"
            aria-label="Download TOON file"
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
        <pre class="toon-output"><code>{toon}</code></pre>
      </div>
    {:else}
      <div class="output-section placeholder">
        <div class="section-header">
          <h4>TOON Output</h4>
        </div>
        <div class="placeholder-content">
          <p>Converted TOON will appear here</p>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .json-to-toon {
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

  .input-mode-selector {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .mode-button {
    flex: 1;
    padding: 0.75rem 1rem;
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--text-primary, #333);
    border: 2px solid var(--border-color, #ddd);
    border-radius: 8px;
    font-size: 0.95rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .mode-button:hover {
    background-color: var(--bg-tertiary, #e8e8e8);
    border-color: var(--accent-color, #b12424);
  }

  .mode-button.active {
    background-color: var(--accent-color, #b12424);
    color: white;
    border-color: var(--accent-color, #b12424);
  }

  .input-container {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .file-input-label {
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
    border-radius: 8px;
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

  .button-group {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .convert-button {
    padding: 0.75rem 1.5rem;
    background-color: var(--accent-color, #b12424);
    color: white;
    border: none;
    border-radius: 8px;
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

  .clear-button,
  .format-button {
    padding: 0.75rem 1.5rem;
    background-color: var(--bg-secondary, #f5f5f5);
    color: var(--text-primary, #333);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    font-size: 1rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .clear-button:hover:not(:disabled),
  .format-button:hover:not(:disabled) {
    background-color: var(--bg-tertiary, #e8e8e8);
    border-color: var(--accent-color, #b12424);
  }

  .clear-button:disabled,
  .format-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
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

  .token-info {
    padding: 0.75rem;
    background-color: var(--bg-secondary, #e8f5e9);
    border: 1px solid var(--border-color, #c8e6c9);
    border-radius: 8px;
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .token-stats {
    display: flex;
    gap: 2rem;
    flex-wrap: wrap;
  }

  .token-stat {
    color: var(--text-primary, #333);
  }

  .token-stat.savings {
    color: var(--accent-color, #2e7d32);
    font-weight: 600;
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

  .json-input {
    flex: 1;
    padding: 1rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
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

  .json-input:focus {
    outline: none;
    border-color: var(--accent-color, #b12424);
  }

  .json-input:disabled {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
  }

  .file-preview {
    flex: 1;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    overflow: hidden;
  }

  .json-preview {
    margin: 0;
    padding: 1rem;
    height: 100%;
    overflow: auto;
    background-color: var(--bg-secondary, #f5f5f5);
    font-family:
      'Menlo', 'Monaco', 'Lucida Console', 'Liberation Mono',
      'DejaVu Sans Mono', 'Bitstream Vera Sans Mono', 'Courier New', monospace;
    font-size: 0.9rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
    color: var(--text-primary, #333);
  }

  .output-section.placeholder {
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
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

  .toon-output {
    flex: 1;
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
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

  .toon-output code {
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
    border-radius: 8px;
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
    border-radius: 8px;
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

  @media screen and (max-width: 768px) {
    .content-container {
      grid-template-columns: 1fr;
    }

    .input-section,
    .output-section {
      min-height: 300px;
    }

    .token-stats {
      flex-direction: column;
      gap: 0.5rem;
    }
  }
</style>
