export interface PageIndexDocument {
  id: string
  filename: string
  title: string
  status: 'processing' | 'ready' | 'error'
  page_count: number | null
  node_count: number | null
  created_at: number
  error: string | null
}

export interface PageIndexNode {
  id: string
  title: string
  page_start: number
  page_end: number
  summary: string
  children: PageIndexNode[]
}

export interface PageIndexListResponse {
  success: boolean
  documents: PageIndexDocument[]
  error?: string
}

export interface PageIndexDetailResponse {
  success: boolean
  document: PageIndexDocument
  tree: PageIndexNode[]
  error?: string
}

export interface PageIndexUploadedDocument {
  id: string
  filename: string
}

export interface PageIndexUploadResponse {
  success: boolean
  message: string
  documents: PageIndexUploadedDocument[]
  error?: string
}

export interface PageIndexDeleteResponse {
  success: boolean
  error?: string
}

export interface PageIndexWsMessage {
  status: 'info' | 'processing' | 'completed' | 'error' | 'log'
  message: string
  document_id: string
  success?: boolean
}
