export interface AgentChatRequest {
  message: string
  conversation_id?: string
}

export interface AgentStreamEvent {
  type: 'status' | 'tool_call' | 'tool_result' | 'text_chunk' | 'done' | 'error'
  status?: string
  message?: string
  tool_name?: string
  arguments?: string
  success?: boolean
  result?: string
  text?: string
  conversation_id?: string
  tool_calls?: Array<{
    tool_name: string
    result: string
  }>
  usage?: {
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  }
}

export interface FileAttachment {
  name: string
  type: 'text' | 'pdf' | 'image' | 'audio'
  content?: string // For text/pdf, store the content
  size?: number
}

export interface ChatMessage {
  id: string // Unique ID for each message
  role: 'user' | 'assistant' | 'status' | 'tool'
  content: string
  timestamp: number
  toolName?: string
  statusType?: string
  attachments?: FileAttachment[]
}

export interface LlamaServerStatus {
  active: boolean
  port: number
}

export interface LlamaServerResponse {
  success: boolean
  message: string
}

export interface AgentConfig {
  enabled_tools: string[]
  chromadb?: {
    collection: string
    embedding_model: string
  }
}

export interface AgentConfigResponse {
  success: boolean
  message: string
}

export interface Collection {
  id: string
  name: string
  metadata?: Record<string, string>
  count?: number
}

export interface ModelInfo {
  name: string
  size?: string
  modified?: string
}

export interface ChromaDBResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export type ToolCategory =
  | 'web'
  | 'financial'
  | 'database'
  | 'search'
  | 'file'
  | 'communication'
  | 'development'
  | 'utility'

export interface ToolInfo {
  id: string
  name: string
  tool_type: 'financial_data' | 'website_check' | 'chroma_db'
  description: string
  category: ToolCategory
  icon: string // Material Icon name
}

export interface ModelCapabilities {
  vision: boolean
  audio: boolean
}
