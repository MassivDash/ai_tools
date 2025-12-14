import { z } from 'zod'

// Text to Tokens Request Schema
export const TextToTokensRequestSchema = z.object({
  text: z.string().min(1, 'Text is required')
})
