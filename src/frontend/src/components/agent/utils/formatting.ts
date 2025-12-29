// Format tool name for display
export const formatToolName = (toolName: string): string => {
  return toolName
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ')
}

// Generate unique ID for messages
export const generateMessageId = (): string => {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`
}

// Clean text for speech (remove markdown symbols, emojis, etc.)
export const cleanTextForSpeech = (text: string): string => {
  if (!text) return ''
  
  return text
    // Remove headers (# Title)
    .replace(/^#+\s+/gm, '')
    // Remove bold/italic markers (**text**, *text*, __text__, _text_)
    .replace(/(\*\*|__)(.*?)\1/g, '$2')
    .replace(/(\*|_)(.*?)\1/g, '$2')
    // Remove code blocks (```code```)
    .replace(/```[\s\S]*?```/g, 'Code block')
    // Remove inline code (`code`)
    .replace(/`([^`]+)`/g, '$1')
    // Remove links ([text](url)) -> text
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    // Remove images (![alt](url)) -> nothing or alt
    .replace(/!\[([^\]]*)\]\([^)]+\)/g, '$1')
    // Remove HTML tags
    .replace(/<[^>]*>/g, '')
    // Remove emojis (approximate ranges)
    .replace(/[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]/gu, '')
    // Collapse whitespace
    .replace(/\s+/g, ' ')
    .trim()
}
