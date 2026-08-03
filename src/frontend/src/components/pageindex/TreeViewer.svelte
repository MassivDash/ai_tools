<script lang="ts">
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type {
    PageIndexDocument,
    PageIndexNode,
    PageIndexDetailResponse
  } from '@types'
  import MaterialIcon from '@ui/MaterialIcon.svelte'

  // Top-level usage: pass the selected document to fetch and render its tree.
  export let document: PageIndexDocument | null = null

  // Recursive usage (via <svelte:self>): pass a single node + its depth.
  export let node: PageIndexNode | null = null
  export let depth: number = 0

  let tree: PageIndexNode[] = []
  let loading = false
  let error = ''

  // Top-level chapters (depth 0) start expanded, anything nested deeper
  // starts collapsed to keep the outline readable at a glance.
  let expanded = depth === 0

  const loadTree = async (doc: PageIndexDocument) => {
    loading = true
    error = ''
    try {
      const response = await axiosBackendInstance.get<PageIndexDetailResponse>(
        `pageindex/documents/${doc.id}`
      )
      if (response.data.success) {
        tree = response.data.tree
      } else {
        error = response.data.error || 'Failed to load document structure'
      }
    } catch (err: any) {
      console.error('Error loading document tree:', err)
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to load document structure'
    } finally {
      loading = false
    }
  }

  $: if (document && document.status === 'ready') {
    loadTree(document)
  }

  const toggleExpanded = () => {
    expanded = !expanded
  }

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      toggleExpanded()
    }
  }
</script>

{#if document}
  <div class="tree-viewer">
    <h3>Table of Contents</h3>
    {#if document.status === 'processing'}
      <div class="placeholder">Indexing in progress…</div>
    {:else if document.status === 'error'}
      <div class="placeholder error-placeholder">
        {document.error || 'An error occurred while indexing this book.'}
      </div>
    {:else if loading}
      <div class="placeholder">Loading table of contents…</div>
    {:else if error}
      <div class="placeholder error-placeholder">{error}</div>
    {:else if tree.length === 0}
      <div class="placeholder">No structure was extracted for this book.</div>
    {:else}
      <div class="tree-root">
        {#each tree as childNode (childNode.id)}
          <svelte:self node={childNode} depth={0} />
        {/each}
      </div>
    {/if}
  </div>
{:else if node}
  <div class="tree-node">
    <div
      class="node-row"
      role="button"
      tabindex="0"
      onclick={toggleExpanded}
      onkeydown={handleKeydown}
      aria-expanded={expanded}
    >
      {#if node.children && node.children.length > 0}
        <MaterialIcon
          name={expanded ? 'chevron-down' : 'chevron-right'}
          width="18"
          height="18"
          class="chevron"
        />
      {:else}
        <span class="chevron-spacer"></span>
      {/if}
      <span class="node-title">{node.title}</span>
      <span class="node-pages">pp. {node.page_start}–{node.page_end}</span>
    </div>
    {#if expanded}
      {#if node.summary}
        <p class="node-summary">{node.summary}</p>
      {/if}
      {#if node.children && node.children.length > 0}
        <div class="node-children">
          {#each node.children as child (child.id)}
            <svelte:self node={child} depth={depth + 1} />
          {/each}
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .tree-viewer {
    margin-top: 1rem;
  }

  .tree-viewer h3 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
    transition: color 0.3s ease;
  }

  .placeholder {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary);
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  .placeholder.error-placeholder {
    color: #f44336;
    border-color: rgba(244, 67, 54, 0.3);
  }

  .tree-root {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .tree-node {
    display: flex;
    flex-direction: column;
  }

  .node-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.5rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .node-row:hover {
    background: var(--bg-secondary);
  }

  :global(.chevron) {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .chevron-spacer {
    display: inline-block;
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .node-title {
    color: var(--text-primary);
    font-weight: 600;
    flex: 1;
    transition: color 0.3s ease;
  }

  .node-pages {
    font-size: 0.8rem;
    color: var(--text-secondary);
    white-space: nowrap;
    transition: color 0.3s ease;
  }

  .node-summary {
    margin: 0.25rem 0 0.5rem 1.75rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.5;
    transition: color 0.3s ease;
  }

  .node-children {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-left: 1.25rem;
    padding-left: 0.75rem;
    border-left: 1px solid var(--border-color);
    transition: border-color 0.3s ease;
  }
</style>
