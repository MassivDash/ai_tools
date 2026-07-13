import { z } from 'zod'

export const ContestantJoinSchema = z.object({
  name: z.string().trim().min(1, 'Name is required'),
  age: z.coerce
    .number()
    .int()
    .positive('Age must be valid')
    .max(120, 'Age must be realistic')
})

export type ContestantJoinPayload = z.infer<typeof ContestantJoinSchema>
