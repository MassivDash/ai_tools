import type { ToolInfo } from '@types'

export interface ToolEntry {
  toolType: string
  category: string
  icon: string
  displayName: string
  description: string
}

export const formatLabel = (value: string): string =>
  value.charAt(0).toUpperCase() + value.slice(1).replace(/_/g, ' ')

// Collapse tools sharing a tool_type into a single entry (e.g. Gmail read/write)
export const deriveToolEntries = (tools: ToolInfo[]): ToolEntry[] => {
  const byType = new Map<string, ToolInfo[]>()
  for (const tool of tools) {
    const list = byType.get(tool.tool_type) || []
    list.push(tool)
    byType.set(tool.tool_type, list)
  }
  return Array.from(byType.entries()).map(([toolType, entries]) => ({
    toolType,
    category: entries[0].category || 'other',
    icon: entries[0].icon,
    displayName: entries.length > 1 ? formatLabel(toolType) : entries[0].name,
    description:
      entries.length > 1
        ? entries.map((t) => t.description).join('. ')
        : entries[0].description
  }))
}
