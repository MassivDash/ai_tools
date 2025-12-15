import { marked } from 'marked'

// Configure marked on first import
if (marked && typeof marked.setOptions === 'function') {
  try {
    marked.setOptions({
      breaks: true,
      gfm: true
    })
  } catch (e) {
    console.error('Failed to configure marked:', e)
  }
}

export const renderMarkdown = (content: string): string => {
  try {
    if (marked && typeof marked.parse === 'function') {
      return marked.parse(content) as string
    }
    // Fallback: simple markdown-like formatting if marked is not available
    return content
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\n/g, '<br>')
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.*?)\*/g, '<em>$1</em>')
      .replace(/`(.*?)`/g, '<code>$1</code>')
  } catch (e) {
    console.error('Failed to render markdown:', e)
    return content
  }
}
