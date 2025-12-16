import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { ToolInfo } from '../types'

// Cache for tool information
let toolsCache: ToolInfo[] | null = null
let toolsCachePromise: Promise<ToolInfo[]> | null = null

/**
 * Fetch available tools from the backend
 */
export const fetchAvailableTools = async (): Promise<ToolInfo[]> => {
  // Return cached result if available
  if (toolsCache) {
    return toolsCache
  }

  // Return existing promise if fetch is in progress
  if (toolsCachePromise) {
    return toolsCachePromise
  }

  // Start new fetch
  toolsCachePromise = axiosBackendInstance
    .get<ToolInfo[]>('agent/tools')
    .then((response) => {
      toolsCache = response.data
      return response.data
    })
    .catch((err) => {
      console.error('❌ Failed to fetch available tools:', err)
      // Return empty array on error
      return []
    })
    .finally(() => {
      // Clear promise after completion
      toolsCachePromise = null
    })

  return toolsCachePromise
}

/**
 * Get tool icon from tool metadata if available, otherwise use pattern matching
 */
export const getToolIconFromMetadata = async (
  toolName: string | undefined
): Promise<string> => {
  if (!toolName) return 'wrench'

  try {
    const tools = await fetchAvailableTools()
    const tool = tools.find(
      (t) =>
        t.name.toLowerCase() === toolName.toLowerCase() ||
        t.tool_type.toLowerCase().replace('_', ' ') === toolName.toLowerCase() ||
        toolName.toLowerCase().includes(t.name.toLowerCase()) ||
        toolName.toLowerCase().includes(t.tool_type.toLowerCase().replace('_', ''))
    )

    if (tool && tool.icon) {
      return tool.icon
    }
  } catch (err) {
    // Fall back to pattern matching if fetch fails
    console.warn('⚠️ Could not fetch tool metadata, using pattern matching:', err)
  }

  // Fall back to pattern matching
  return getToolIcon(toolName)
}

/**
 * Get tool icon based on tool name or tool type
 * This function tries to match the tool name to known patterns
 * and falls back to a default icon if no match is found
 */
export const getToolIcon = (toolName: string | undefined): string => {
  if (!toolName) return 'wrench'

  const name = toolName.toLowerCase()

  // Match by function name patterns
  if (name.includes('check_website') || name.includes('website_check') || name.includes('website')) {
    return 'web'
  }
  if (name.includes('financial') || name.includes('sql_query')) {
    return 'currency-usd'
  }
  if (name.includes('chromadb') || name.includes('chroma') || name.includes('database')) {
    return 'database'
  }
  if (name.includes('search') || name.includes('query')) {
    return 'magnify'
  }
  if (name.includes('read') || name.includes('file')) {
    return 'file-document'
  }
  if (name.includes('write') || name.includes('save')) {
    return 'file-edit'
  }
  if (name.includes('delete') || name.includes('remove')) {
    return 'delete'
  }
  if (name.includes('update') || name.includes('edit')) {
    return 'pencil'
  }
  if (name.includes('create') || name.includes('add') || name.includes('new')) {
    return 'plus-circle'
  }
  if (name.includes('send') || name.includes('post')) {
    return 'send'
  }
  if (name.includes('get') || name.includes('fetch') || name.includes('retrieve')) {
    return 'download'
  }

  // Default icon
  return 'wrench'
}

/**
 * Get tool icon with tool metadata support
 * This function uses tool metadata from the backend for accurate icon selection
 * @deprecated Use getToolIconFromMetadata instead
 */
export const getToolIconWithMetadata = getToolIconFromMetadata

/**
 * Clear the tools cache (useful for testing or when tools are updated)
 */
export const clearToolsCache = () => {
  toolsCache = null
  toolsCachePromise = null
}

