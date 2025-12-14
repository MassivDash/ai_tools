import { z } from 'zod'

export const JsonToToonRequestSchema = z.object({
  json: z.string().min(1, { message: 'JSON content cannot be empty' }),
  count_tokens: z.boolean().default(false)
})
