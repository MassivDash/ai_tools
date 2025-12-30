<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { QueryRequest, QueryResponse, ChromaDBResponse } from '@types'
  import { QueryRequestSchema } from '@validation/chromadb.ts'
  import Button from '../ui/Button.svelte'
  import Input from '../ui/Input.svelte'

  export let selectedCollection: string | null = null

  let queryText = ''
  let nResults = 10
  let loading = false
  let error = ''
  let results: QueryResponse | null = null

  const performQuery = async () => {
    loading = true
    error = ''
    results = null

    try {
      // Validate with Zod
      const validationResult = QueryRequestSchema.safeParse({
        collection: selectedCollection,
        query_texts: [queryText.trim()],
        n_results: nResults
      })

      if (!validationResult.success) {
        const firstError = validationResult.error.issues[0]
        error = firstError.message
        loading = false
        return
      }

      const request: QueryRequest = validationResult.data

      const response = await axiosBackendInstance.post<
        ChromaDBResponse<QueryResponse>
      >('chromadb/query', request)

      if (response.data.success && response.data.data) {
        results = response.data.data
      } else {
        error = response.data.error || 'Failed to perform query'
      }
    } catch (err: any) {
      console.error('Error querying:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to perform query'
    } finally {
      loading = false
    }
  }

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      performQuery()
    }
  }
</script>

<div class="query-interface">
  <h3>Search Collection</h3>

  {#if !selectedCollection}
    <div class="warning">⚠️ Please select a collection first to search</div>
  {:else}
    <div class="query-form">
      <div class="form-group">
        <label for="query-text">Query Text</label>
        <Input
          id="query-text"
          bind:value={queryText}
          placeholder="Enter your search query..."
          onkeypress={handleKeyPress}
          disabled={loading}
        />
      </div>

      <div class="form-group">
        <label for="n-results">Number of Results</label>
        <Input
          id="n-results"
          type="number"
          bind:value={nResults}
          min="1"
          max="100"
          disabled={loading}
        />
      </div>

      <Button
        onclick={performQuery}
        disabled={loading || !queryText.trim() || !selectedCollection}
        variant="primary"
      >
        {loading ? 'Searching...' : 'Search'}
      </Button>
    </div>

    {#if error}
      <div class="error-message">{error}</div>
    {/if}

    {#if results}
      <div class="results">
        <h4>Results ({results.ids[0]?.length || 0})</h4>
        {#if results.ids[0] && results.ids[0].length > 0}
          <div class="results-list">
            {#each results.ids[0] as id, index}
              <div class="result-item">
                <div class="result-header">
                  <span class="result-id">ID: {id}</span>
                  {#if results.distances?.[0]?.[index] !== undefined}
                    <span class="result-distance">
                      Distance: {results.distances[0][index].toFixed(4)}
                    </span>
                  {/if}
                </div>

                {#if results.documents?.[0]?.[index]}
                  <div class="result-document">
                    <strong>Document:</strong>
                    <p>{results.documents[0][index]}</p>
                  </div>
                {/if}

                {#if results.metadatas?.[0]?.[index]}
                  <div class="result-metadata">
                    <strong>Metadata:</strong>
                    <pre>{JSON.stringify(
                        results.metadatas[0][index],
                        null,
                        2
                      )}</pre>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="no-results">No results found</div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .query-interface {
    margin-bottom: 2rem;
  }

  .query-interface h3 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
    transition: color 0.3s ease;
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

  .query-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-group label {
    font-weight: 600;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .error-message {
    padding: 1rem;
    background: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
    color: var(--accent-color, #c33);
    margin-bottom: 1rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .results {
    margin-top: 1.5rem;
    padding: 1rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .results h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .result-item {
    padding: 1rem;
    background: var(--bg-secondary);
    border-radius: 8px;
    border-left: 3px solid #4a90e2;
    transition: background-color 0.3s ease;
  }

  .result-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--border-color);
    transition: border-color 0.3s ease;
  }

  .result-id {
    font-weight: 600;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .result-distance {
    font-size: 0.9rem;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }

  .result-document,
  .result-metadata {
    margin-top: 0.75rem;
  }

  .result-document strong,
  .result-metadata strong {
    display: block;
    margin-bottom: 0.5rem;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .result-document p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.6;
    white-space: pre-wrap;
    transition: color 0.3s ease;
  }

  .result-metadata pre {
    margin: 0;
    padding: 0.5rem;
    background: var(--bg-primary);
    border-radius: 8px;
    font-size: 0.85rem;
    overflow-x: auto;
    transition: background-color 0.3s ease;
  }

  .no-results {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary);
    transition: color 0.3s ease;
  }
</style>
