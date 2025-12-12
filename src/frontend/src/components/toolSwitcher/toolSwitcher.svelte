<script lang="ts">
  import UrlToMarkdown from '../urlToMarkdown/urlToMarkdown.svelte'

  type ToolType = 'url-to-markdown' | 'html-to-markdown' | 'pdf-to-markdown' | null

  let selectedTool: ToolType = null

  const tools = [
    {
      id: 'url-to-markdown' as ToolType,
      name: 'URL to Markdown',
      description: 'Convert web pages to markdown format',
      icon: '🔗'
    },
    {
      id: 'html-to-markdown' as ToolType,
      name: 'HTML to Markdown',
      description: 'Paste HTML and convert to markdown',
      icon: '📄'
    },
    {
      id: 'pdf-to-markdown' as ToolType,
      name: 'PDF to Markdown',
      description: 'Upload PDF files and convert to markdown',
      icon: '📑'
    }
  ]

  const selectTool = (toolId: ToolType) => {
    selectedTool = toolId
  }

  const closeTool = () => {
    selectedTool = null
  }
</script>

<div class="tool-switcher">
  <div class="tools-header">
    <h2>AI Tools</h2>
    <p class="subtitle">Select a tool to get started</p>
  </div>

  {#if selectedTool === null}
    <div class="tools-grid">
      {#each tools as tool}
        <button
          class="tool-card"
          onclick={() => selectTool(tool.id)}
          type="button"
        >
          <div class="tool-icon">{tool.icon}</div>
          <h3 class="tool-name">{tool.name}</h3>
          <p class="tool-description">{tool.description}</p>
        </button>
      {/each}
    </div>
  {:else}
    <div class="tool-container">
      <div class="tool-header">
        <button onclick={closeTool} class="back-button" type="button">
          ← Back to Tools
        </button>
        <h3>
          {tools.find((t) => t.id === selectedTool)?.name || 'Tool'}
        </h3>
      </div>
      <div class="tool-content">
        {#if selectedTool === 'url-to-markdown'}
          <UrlToMarkdown />
        {:else if selectedTool === 'html-to-markdown'}
          <div class="placeholder-tool">
            <p>🚧 HTML to Markdown tool coming soon</p>
            <p class="hint">This tool will allow you to paste HTML content and convert it to markdown format.</p>
          </div>
        {:else if selectedTool === 'pdf-to-markdown'}
          <div class="placeholder-tool">
            <p>🚧 PDF to Markdown tool coming soon</p>
            <p class="hint">This tool will allow you to upload PDF files and convert them to markdown format.</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .tool-switcher {
    width: 100%;
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem 1rem;
  }

  .tools-header {
    text-align: center;
    margin-bottom: 3rem;
  }

  .tools-header h2 {
    font-size: 2.5rem;
    margin: 0 0 0.5rem 0;
    color: #100f0f;
  }

  .subtitle {
    font-size: 1.1rem;
    color: #666;
    margin: 0;
  }

  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 2rem;
    margin-top: 2rem;
  }

  .tool-card {
    background: white;
    border: 2px solid #e0e0e0;
    border-radius: 12px;
    padding: 2rem;
    cursor: pointer;
    transition: all 0.3s ease;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .tool-card:hover {
    border-color: #b12424;
    transform: translateY(-4px);
    box-shadow: 0 8px 16px rgba(177, 36, 36, 0.2);
  }

  .tool-card:active {
    transform: translateY(-2px);
  }

  .tool-icon {
    font-size: 3rem;
    margin-bottom: 0.5rem;
  }

  .tool-name {
    margin: 0;
    font-size: 1.5rem;
    color: #100f0f;
  }

  .tool-description {
    margin: 0;
    color: #666;
    font-size: 0.95rem;
    line-height: 1.5;
  }

  .tool-container {
    background: white;
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 2rem;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .tool-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 2px solid #f0f0f0;
  }

  .back-button {
    padding: 0.5rem 1rem;
    background-color: #f5f5f5;
    color: #666;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .back-button:hover {
    background-color: #e8e8e8;
    color: #333;
  }

  .tool-header h3 {
    margin: 0;
    font-size: 1.8rem;
    color: #100f0f;
  }

  .tool-content {
    min-height: 400px;
  }

  .placeholder-tool {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 400px;
    text-align: center;
    color: #666;
  }

  .placeholder-tool p {
    margin: 0.5rem 0;
    font-size: 1.1rem;
  }

  .placeholder-tool .hint {
    font-size: 0.9rem;
    color: #999;
    max-width: 500px;
  }

  @media screen and (max-width: 768px) {
    .tools-header h2 {
      font-size: 2rem;
    }

    .tools-grid {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }

    .tool-card {
      padding: 1.5rem;
    }

    .tool-container {
      padding: 1rem;
    }
  }
</style>

