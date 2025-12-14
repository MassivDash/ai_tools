import { z } from 'zod'

// Llama Server Config Request Schema
// All fields are optional to match the backend structure
export const LlamaConfigRequestSchema = z.object({
  hf_model: z
    .string()
    .trim()
    .min(1, 'HuggingFace model cannot be empty')
    .optional(),
  ctx_size: z
    .number()
    .int()
    .positive('Context size must be greater than 0')
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
  model: z
    .string()
    .trim()
    .min(1, 'Model path cannot be empty')
    .nullable()
    .optional()
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
  model: string
}) => {
  const payload: Record<string, any> = {}

  // hf_model - required, must be non-empty after trim
  const trimmedHfModel = values.hf_model.trim()
  if (trimmedHfModel) {
    payload.hf_model = trimmedHfModel
  }

  // ctx_size - must be > 0 if provided
  if (values.ctx_size > 0) {
    payload.ctx_size = values.ctx_size
  }

  // Optional numeric fields - only include if not empty string
  if (values.threads !== '') {
    payload.threads = values.threads
  }
  if (values.threads_batch !== '') {
    payload.threads_batch = values.threads_batch
  }
  if (values.predict !== '') {
    payload.predict = values.predict
  }
  if (values.batch_size !== '') {
    payload.batch_size = values.batch_size
  }
  if (values.ubatch_size !== '') {
    payload.ubatch_size = values.ubatch_size
  }
  if (values.gpu_layers !== '') {
    payload.gpu_layers = values.gpu_layers
  }

  // Optional boolean fields - only include if true
  if (values.flash_attn) {
    payload.flash_attn = values.flash_attn
  }
  if (values.mlock) {
    payload.mlock = values.mlock
  }
  if (values.no_mmap) {
    payload.no_mmap = values.no_mmap
  }

  // model - optional, must be non-empty after trim if provided
  const trimmedModel = values.model.trim()
  if (trimmedModel) {
    payload.model = trimmedModel
  }

  return payload
}
