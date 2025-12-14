import { z } from 'zod'

export const HtmlToMarkdownRequestSchema = z.object({
  html: z.string().min(1, { message: 'HTML content cannot be empty' }),
  extract_body: z.boolean().default(true),
  enable_preprocessing: z.boolean().default(false),
  remove_navigation: z.boolean().default(false),
  remove_forms: z.boolean().default(false),
  preprocessing_preset: z
    .enum(['minimal', 'standard', 'aggressive'])
    .nullable()
    .default(null),
  count_tokens: z.boolean().default(false)
})

