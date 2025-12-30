<script lang="ts">
  import HtmlToMarkdown from '../htmlToMarkdown/htmlToMarkdown.svelte'
  import JsonToToon from '../jsonToToon/jsonToToon.svelte'
  import ParquetToTxt from '../parquetToTxt/parquetToTxt.svelte'
  import PdfToMarkdown from '../pdfToMarkdown/pdfToMarkdown.svelte'
  import TextToTokens from '../textToTokens/textToTokens.svelte'
  import UrlToMarkdown from '../urlToMarkdown/urlToMarkdown.svelte'
  import Card from '../../ui/Card.svelte'
  import Button from '../../ui/Button.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'

  type ToolType =
    | 'url-to-markdown'
    | 'html-to-markdown'
    | 'json-to-toon'
    | 'parquet-to-txt'
    | 'pdf-to-markdown'
    | 'text-to-tokens'
    | null

  let selectedTool: ToolType = null

  const tools = [
    {
      id: 'url-to-markdown' as ToolType,
      name: 'URL to Markdown',
      description: 'Convert web pages to markdown format',
      icon: 'link-variant'
    },
    {
      id: 'html-to-markdown' as ToolType,
      name: 'HTML to Markdown',
      description: 'Paste HTML and convert to markdown',
      icon: 'language-html'
    },
    {
      id: 'json-to-toon' as ToolType,
      name: 'JSON to TOON',
      description: 'Convert JSON to TOON format for LLMs',
      icon: 'code-json'
    },
    {
      id: 'parquet-to-txt' as ToolType,
      name: 'Parquet to TXT',
      description:
        'Combine and convert parquet files to text for Imatrix Quantization',
      icon: 'table-large'
    },
    {
      id: 'pdf-to-markdown' as ToolType,
      name: 'PDF to Markdown',
      description: 'Upload PDF files and convert to markdown',
      icon: 'file-pdf-box'
    },
    {
      id: 'text-to-tokens' as ToolType,
      name: 'Text to Tokens',
      description: 'Count tokens in any text using GPT-2 tokenizer',
      icon: 'abacus'
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
          class="tool-card-wrapper"
          onclick={() => selectTool(tool.id)}
          type="button"
        >
          <Card class="tool-card-content" variant="elevated">
            <div class="tool-icon-wrapper">
              <MaterialIcon name={tool.icon} width="48" height="48" />
            </div>
            <h3 class="tool-name">{tool.name}</h3>
            <p class="tool-description">{tool.description}</p>
          </Card>
        </button>
      {/each}
    </div>
  {:else}
    <div class="tool-container">
      <div class="tool-header">
        <Button variant="secondary" onclick={closeTool}>
          <MaterialIcon name="arrow-left" width="20" height="20" />
          Back to Tools
        </Button>
        <h3>
          {tools.find((t) => t.id === selectedTool)?.name || 'Tool'}
        </h3>
      </div>
      <div class="tool-content">
        {#if selectedTool === 'url-to-markdown'}
          <UrlToMarkdown />
        {:else if selectedTool === 'html-to-markdown'}
          <HtmlToMarkdown />
        {:else if selectedTool === 'json-to-toon'}
          <JsonToToon />
        {:else if selectedTool === 'parquet-to-txt'}
          <ParquetToTxt />
        {:else if selectedTool === 'pdf-to-markdown'}
          <PdfToMarkdown />
        {:else if selectedTool === 'text-to-tokens'}
          <TextToTokens />
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .tool-switcher {
    width: 100%;
    max-width: calc(100% - 5rem);
    margin: 0 auto;
    padding: 2rem 1rem;
    margin-bottom: 10rem;
  }

  .tools-header {
    text-align: center;
    margin-bottom: 3rem;
  }

  .tools-header h2 {
    font-size: 2.5rem;
    margin: 0 0 0.5rem 0;
    color: var(--md-on-surface);
  }

  .subtitle {
    font-size: 1.1rem;
    color: var(--md-on-surface-variant);
    margin: 0;
  }

  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 2rem;
    row-gap: 4rem;
    margin-top: 2rem;
  }

  /* Interactive wrapper for Card */
  .tool-card-wrapper {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: inherit;
    width: 100%;
    height: 100%;
    display: flex; /* To make card fill height */
  }

  .tool-card-wrapper:hover :global(.card) {
    transform: translateY(-4px);
    box-shadow: 0 8px 16px -4px var(--md-shadow, rgba(0, 0, 0, 0.1));
    border-color: var(--md-primary);
  }

  :global(.tool-card-content) {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 1rem;
    justify-content: center;
    min-height: 200px;
  }

  .tool-icon-wrapper {
    color: var(--md-primary);
    margin-bottom: 0.5rem;
    padding: 1rem;
    background-color: var(--md-primary-container);
    border-radius: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .tool-name {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 500;
    color: var(--md-on-surface);
  }

  .tool-description {
    margin: 0;
    color: var(--md-on-surface-variant);
    font-size: 0.9rem;
    line-height: 1.5;
  }

  .tool-container {
    background: var(--md-surface);
    border-radius: 12px;
    padding: 2rem;
    box-shadow: 0 2px 8px -2px var(--md-shadow);
  }

  .tool-header {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--md-outline-variant);
  }

  .tool-header h3 {
    margin: 0;
    font-size: 1.5rem;
    color: var(--md-on-surface);
    font-weight: 500;
  }

  .tool-content {
    min-height: 400px;
  }

  @media screen and (max-width: 768px) {
    .tools-header h2 {
      font-size: 2rem;
    }

    .tool-container {
      padding: 1rem;
    }
  }
</style>
