import { marked } from 'marked'
import markedKatex from 'marked-katex-extension'

// Configure marked with katex extension
if (marked && typeof marked.use === 'function') {
  try {
    marked.use(
      markedKatex({
        throwOnError: false
      })
    )

    marked.setOptions({
      breaks: true,
      gfm: true
    })
  } catch (e) {
    console.error('Failed to configure marked:', e)
  }
}

const preprocessMath = (content: string): string => {
  if (!content) return content
  // Replace \[ ... \] with $$ ... $$ for block math
  // We use a function replacement to handle the capture group correctly and avoid issues with $ literals
  let processed = content.replace(/\\\[([\s\S]*?)\\\]/g, (_, equation) => {
    return '$$' + equation + '$$'
  })

  // Replace \( ... \) with $ ... $ for inline math
  processed = processed.replace(/\\\(([\s\S]*?)\\\)/g, (_, equation) => {
    return '$' + equation + '$'
  })

  return processed
}

export const renderMarkdown = (content: string): string => {
  try {
    if (marked && typeof marked.parse === 'function') {
      const processedContent = preprocessMath(content)
      return marked.parse(processedContent) as string
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
