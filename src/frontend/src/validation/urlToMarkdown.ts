import { z } from 'zod'

// URL validation schema
const urlSchema = z
  .string()
  .min(1, 'URL is required')
  .url({ message: 'Please enter a valid URL' })

// Preprocessing preset enum
export const PreprocessingPresetSchema = z.enum([
  'minimal',
  'standard',
  'aggressive'
])

// URL to Markdown Request Schema
export const UrlToMarkdownRequestSchema = z.object({
  url: urlSchema,
  extract_body: z.boolean().default(false),
  enable_preprocessing: z.boolean().default(false),
  remove_navigation: z.boolean().default(false),
  remove_forms: z.boolean().default(false),
  preprocessing_preset: PreprocessingPresetSchema.nullable().optional(),
  follow_links: z.boolean().default(false),
  count_tokens: z.boolean().default(false)
})
