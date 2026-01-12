import { z } from 'zod'

// ChromaDB Config Request Schema
export const ChromaDBConfigRequestSchema = z.object({
  embedding_model: z.string().trim().min(1, 'Embedding model cannot be empty'),
  query_model: z
    .string()
    .trim()
    .min(1, 'Query model cannot be empty')
    .optional(),
  chunk_size: z.number().int().min(1).optional(),
  chunk_overlap: z.number().int().min(0).optional()
})

// Helper function to build the payload from form values
export const buildChromaDBConfigPayload = (values: {
  embedding_model: string
  query_model?: string
  chunk_size?: number
  chunk_overlap?: number
}) => {
  const payload: Record<string, any> = {}

  // embedding_model - required, must be non-empty after trim
  const trimmedEmbeddingModel = values.embedding_model.trim()
  if (trimmedEmbeddingModel) {
    payload.embedding_model = trimmedEmbeddingModel
  }

  // query_model - optional, defaults to embedding_model if not provided
  if (values.query_model) {
    const trimmedQueryModel = values.query_model.trim()
    if (trimmedQueryModel) {
      payload.query_model = trimmedQueryModel
    }
  }

  if (values.chunk_size !== undefined) {
    payload.chunk_size = values.chunk_size
  }
  
  if (values.chunk_overlap !== undefined) {
    payload.chunk_overlap = values.chunk_overlap
  }

  return payload
}
