import { z } from 'zod'

// Distance metric enum
export const DistanceMetricSchema = z.enum(['cosine', 'l2', 'ip'])

// Create Collection Request Schema
export const CreateCollectionRequestSchema = z.object({
  name: z
    .string()
    .min(1, 'Collection name is required')
    .max(100, 'Collection name is too long (max 100 characters)')
    .refine(
      (val) => {
        // ChromaDB collection names must be valid identifiers
        // Allow alphanumeric, underscores, and hyphens
        const trimmed = val.trim()
        if (trimmed.length === 0) return false
        // Check if it contains only valid characters
        const validPattern = /^[a-zA-Z0-9_-]+$/
        return validPattern.test(trimmed)
      },
      {
        message:
          'Collection name can only contain alphanumeric characters, underscores, and hyphens'
      }
    ),
  metadata: z.record(z.string(), z.string()).optional(),
  distance_metric: DistanceMetricSchema.optional()
})

// Query Request Schema
export const QueryRequestSchema = z.object({
  collection: z.string().min(1, 'Collection name is required'),
  query_texts: z
    .array(z.string().min(1, 'Query text cannot be empty'))
    .min(1, 'At least one query text is required'),
  n_results: z.number().int().min(1).max(100).optional(),
  where_clause: z.record(z.string(), z.any()).optional()
})

// File upload validation (for FormData, we validate the collection name separately)
export const DocumentUploadSchema = z.object({
  collection: z.string().min(1, 'Collection name is required')
})

// File type validation
const SUPPORTED_FILE_TYPES = [
  'application/pdf',
  'text/markdown',
  'text/plain',
  'text/mdx'
] as const

const SUPPORTED_EXTENSIONS = ['.pdf', '.md', '.mdx', '.txt'] as const

export const validateFileType = (file: File): boolean => {
  // Check MIME type
  if (SUPPORTED_FILE_TYPES.includes(file.type as any)) {
    return true
  }
  // Check file extension as fallback
  const fileName = file.name.toLowerCase()
  return SUPPORTED_EXTENSIONS.some((ext) => fileName.endsWith(ext))
}

export const FileValidationSchema = z
  .instanceof(File)
  .refine(validateFileType, {
    message:
      'File type not supported. Only PDF, Markdown (.md, .mdx), and Text (.txt) files are allowed.'
  })

