<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import { TextToTokensRequestSchema } from '@validation/textToTokens.ts'

  interface TokenResponse {
    token_count: number
    character_count: number
    word_count: number
  }

  let text = ''
  let loading = false
  let error = ''
  let tokenCount = 0
  let characterCount = 0
  let wordCount = 0

  const convertTextToTokens = async () => {
    if (!text.trim()) {
      error = 'Please enter some text'
      return
    }

    loading = true
    error = ''
    tokenCount = 0
    characterCount = 0
    wordCount = 0

    try {
      // Validate with Zod
      const validationResult = TextToTokensRequestSchema.safeParse({
        text: text.trim()
      })

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        loading = false
        return
      }

      const requestData = validationResult.data

      const res = await axiosBackendInstance.post<TokenResponse>(
        'text-to-tokens',
        {
          text: requestData.text
        }
      )

      tokenCount = res.data.token_count
      characterCount = res.data.character_count
      wordCount = res.data.word_count
    } catch (err: any) {
      error =
        err.response?.data?.error || err.message || 'Failed to count tokens'
      tokenCount = 0
      characterCount = 0
      wordCount = 0
    } finally {
      loading = false
    }
  }

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      convertTextToTokens()
    }
  }
</script>

<div class="text-to-tokens">
  <h3>Text to Tokens Converter</h3>
  <div class="input-container">
    <textarea
      bind:value={text}
      placeholder="Enter or paste text to count tokens..."
      onkeydown={handleKeyPress}
      disabled={loading}
      class="text-input"
      rows="10"
    ></textarea>
    <button
      onclick={convertTextToTokens}
      disabled={loading || !text.trim()}
      class="convert-button"
    >
      {loading ? 'Counting...' : 'Count Tokens'}
    </button>
  </div>

  <div class="hint">
    <p>💡 Tip: Press Ctrl+Enter (or Cmd+Enter on Mac) to count tokens</p>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if tokenCount > 0}
    <div class="results-container">
      <h4>Token Count Results:</h4>
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-label">Tokens</div>
          <div class="stat-value">{tokenCount.toLocaleString()}</div>
          <div class="stat-description">GPT-2 tokenizer count</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Words</div>
          <div class="stat-value">{wordCount.toLocaleString()}</div>
          <div class="stat-description">Whitespace-separated words</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Characters</div>
          <div class="stat-value">{characterCount.toLocaleString()}</div>
          <div class="stat-description">Total character count</div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .text-to-tokens {
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
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .text-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    font-size: 1rem;
    font-family: inherit;
    background-color: var(--bg-primary, white);
    color: var(--text-primary, #333);
    resize: vertical;
    min-height: 200px;
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .text-input:focus {
    outline: none;
    border-color: var(--accent-color, #b12424);
  }

  .text-input:disabled {
    background-color: var(--bg-secondary, #f5f5f5);
    cursor: not-allowed;
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
    align-self: flex-start;
  }

  .convert-button:hover:not(:disabled) {
    background-color: var(--accent-hover, #8a1c1c);
  }

  .convert-button:disabled {
    background-color: var(--text-tertiary, #ccc);
    cursor: not-allowed;
  }

  .hint {
    margin-bottom: 1rem;
    padding: 0.5rem;
    background-color: var(--bg-secondary, #f0f0f0);
    border-radius: 8px;
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
  }

  .hint p {
    margin: 0;
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

  .results-container {
    margin-top: 1rem;
    padding: 1.5rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .results-container h4 {
    margin-top: 0;
    margin-bottom: 1rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .stat-card {
    background-color: var(--bg-primary, white);
    border: 1px solid var(--border-color, #ddd);
    border-radius: 6px;
    padding: 1rem;
    text-align: center;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      transform 0.2s ease;
  }

  .stat-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
  }

  .stat-label {
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
    margin-bottom: 0.5rem;
    font-weight: 500;
    transition: color 0.3s ease;
  }

  .stat-value {
    font-size: 2rem;
    font-weight: bold;
    color: var(--accent-color, #b12424);
    margin-bottom: 0.5rem;
    transition: color 0.3s ease;
  }

  .stat-description {
    font-size: 0.8rem;
    color: var(--text-tertiary, #999);
    transition: color 0.3s ease;
  }

  @media screen and (max-width: 768px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
