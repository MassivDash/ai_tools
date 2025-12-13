export interface ChromaDBCollection {
  id: string
  name: string
  metadata?: Record<string, string>
  count?: number
}

export interface ChromaDBResponse<T> {
  success: boolean
  data?: T
  error?: string
  message?: string
}

export interface ChromaDBHealthResponse {
  status: 'healthy' | 'unhealthy'
  version: string
  chromadb: {
    connected: boolean
  }
}

export interface QueryRequest {
  collection: string
  query_texts: string[]
  n_results?: number
  where_clause?: Record<string, any>
}

export interface QueryResponse {
  ids: string[][]
  distances?: number[][]
  documents?: string[][]
  metadatas?: Array<Array<Record<string, any>>>
}

export interface CreateCollectionRequest {
  name: string
  metadata?: Record<string, string>
}

export interface UploadDocumentRequest {
  collection: string
  files: File[]
  chunk_size?: number
  chunk_overlap?: number
}

export interface ProcessingStatus {
  status: 'processing' | 'completed' | 'error'
  progress: number
  message: string
  processed_files: number
  total_files: number
}


