import { z } from 'zod'

// Llama Server Config Request Schema
// All fields are optional to match the backend structure
export const LlamaConfigRequestSchema = z.object({
  hf_model: z.string().trim().optional(),
  ctx_size: z
    .number()
    .int()
    .nonnegative('Context size must be 0 or greater')
    .optional(),
  threads: z.number().int().nullable().optional(),
  threads_batch: z.number().int().nullable().optional(),
  predict: z.number().int().nullable().optional(),
  batch_size: z
    .number()
    .int()
    .positive('Batch size must be greater than 0')
    .nullable()
    .optional(),
  ubatch_size: z
    .number()
    .int()
    .positive('UBatch size must be greater than 0')
    .nullable()
    .optional(),
  flash_attn: z.boolean().nullable().optional(),
  mlock: z.boolean().nullable().optional(),
  no_mmap: z.boolean().nullable().optional(),
  gpu_layers: z
    .number()
    .int()
    .nonnegative('GPU layers must be 0 or greater')
    .nullable()
    .optional(),
  n_cpu_moe: z
    .number()
    .int()
    .nonnegative('N CPU MoE must be 0 or greater')
    .nullable()
    .optional(),
  model: z.string().trim().nullable().optional()
})

// Helper function to build the payload from form values
// This handles the conversion of empty strings to undefined
export const buildLlamaConfigPayload = (values: {
  hf_model: string
  ctx_size: number
  threads: number | ''
  threads_batch: number | ''
  predict: number | ''
  batch_size: number | ''
  ubatch_size: number | ''
  flash_attn: boolean
  mlock: boolean
  no_mmap: boolean
  gpu_layers: number | ''
  n_cpu_moe: number | ''
  model: string
}) => {
  const payload: Record<string, any> = {}

  // hf_model - can be empty if model path is provided
  const trimmedHfModel = values.hf_model.trim()
  payload.hf_model = trimmedHfModel

  // ctx_size - must be >= 0 if provided
  if (values.ctx_size >= 0) {
    payload.ctx_size = values.ctx_size
  }

  // Optional numeric fields - empty input is sent as null so the backend clears it
  const orNull = (v: number | '') => (v === '' ? null : v)
  payload.threads = orNull(values.threads)
  payload.threads_batch = orNull(values.threads_batch)
  payload.predict = orNull(values.predict)
  payload.batch_size = orNull(values.batch_size)
  payload.ubatch_size = orNull(values.ubatch_size)
  payload.gpu_layers = orNull(values.gpu_layers)
  payload.n_cpu_moe = orNull(values.n_cpu_moe)

  // Boolean fields - always sent so unchecking a box actually turns it off
  payload.flash_attn = values.flash_attn
  payload.mlock = values.mlock
  payload.no_mmap = values.no_mmap

  // model - optional, include if provided (even empty string to clear it)
  if (values.model !== undefined) {
    payload.model = values.model.trim()
  }

  return payload
}
