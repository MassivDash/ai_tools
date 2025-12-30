export interface LlamaModelInfo {
  name: string
  path: string
  size?: number
  hf_format?: string
}

export interface OllamaModelInfo {
  name: string
  size?: string
  modified?: string
}

export interface ModelNote {
  id?: number
  platform: string
  model_name: string
  model_path?: string
  is_favorite: boolean
  is_default: boolean
  tags: string[]
  notes?: string
  created_at?: number
  updated_at?: number
}

export interface ModelNoteRequest {
  platform: string
  model_name: string
  model_path?: string
  is_favorite?: boolean
  is_default?: boolean
  tags?: string[]
  notes?: string
}
