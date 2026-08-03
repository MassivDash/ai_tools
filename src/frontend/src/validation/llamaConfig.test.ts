import { describe, expect, test } from 'vitest'
import {
  LlamaConfigRequestSchema,
  buildLlamaConfigPayload
} from './llamaConfig.ts'

type FormValues = Parameters<typeof buildLlamaConfigPayload>[0]

const emptyForm: FormValues = {
  hf_model: '',
  ctx_size: 0,
  threads: '',
  threads_batch: '',
  predict: '',
  batch_size: '',
  ubatch_size: '',
  flash_attn: false,
  mlock: false,
  no_mmap: false,
  gpu_layers: '',
  model: ''
}

describe('LlamaConfigRequestSchema', () => {
  test('accepts an empty object because every field is optional', () => {
    const result = LlamaConfigRequestSchema.safeParse({})
    expect(result.success).toBe(true)
    expect(result.data).toEqual({})
  })

  test('trims the string fields', () => {
    const result = LlamaConfigRequestSchema.safeParse({
      hf_model: '  unsloth/Qwen3-8B-GGUF  ',
      model: '  /models/a.gguf  '
    })
    expect(result.data).toEqual({
      hf_model: 'unsloth/Qwen3-8B-GGUF',
      model: '/models/a.gguf'
    })
  })

  test('accepts null for the nullable fields', () => {
    const result = LlamaConfigRequestSchema.safeParse({
      threads: null,
      threads_batch: null,
      predict: null,
      batch_size: null,
      ubatch_size: null,
      flash_attn: null,
      mlock: null,
      no_mmap: null,
      gpu_layers: null,
      model: null
    })
    expect(result.success).toBe(true)
    expect(result.data?.threads).toBeNull()
    expect(result.data?.gpu_layers).toBeNull()
  })

  test('accepts a fully populated config', () => {
    const result = LlamaConfigRequestSchema.safeParse({
      hf_model: 'unsloth/Qwen3-8B-GGUF',
      ctx_size: 8192,
      threads: 8,
      threads_batch: 4,
      predict: -1,
      batch_size: 2048,
      ubatch_size: 512,
      flash_attn: true,
      mlock: false,
      no_mmap: true,
      gpu_layers: 99,
      model: '/models/a.gguf'
    })
    expect(result.success).toBe(true)
  })

  test('allows a context size of 0 but rejects a negative one', () => {
    expect(LlamaConfigRequestSchema.safeParse({ ctx_size: 0 }).success).toBe(
      true
    )
    const result = LlamaConfigRequestSchema.safeParse({ ctx_size: -1 })
    expect(result.success).toBe(false)
    expect(result.error?.issues[0]?.message).toBe(
      'Context size must be 0 or greater'
    )
    expect(result.error?.issues[0]?.path).toEqual(['ctx_size'])
  })

  test.each([
    ['batch_size', 'Batch size must be greater than 0'],
    ['ubatch_size', 'UBatch size must be greater than 0']
  ])('rejects a %s of 0 with its custom message', (field, message) => {
    const result = LlamaConfigRequestSchema.safeParse({ [field]: 0 })
    expect(result.success).toBe(false)
    expect(result.error?.issues[0]?.message).toBe(message)
    expect(result.error?.issues[0]?.path).toEqual([field])
    expect(LlamaConfigRequestSchema.safeParse({ [field]: 1 }).success).toBe(
      true
    )
  })

  test('allows 0 gpu layers but rejects a negative count', () => {
    expect(LlamaConfigRequestSchema.safeParse({ gpu_layers: 0 }).success).toBe(
      true
    )
    const result = LlamaConfigRequestSchema.safeParse({ gpu_layers: -1 })
    expect(result.success).toBe(false)
    expect(result.error?.issues[0]?.message).toBe(
      'GPU layers must be 0 or greater'
    )
  })

  test('rejects non-integer numbers', () => {
    const result = LlamaConfigRequestSchema.safeParse({ threads: 2.5 })
    expect(result.success).toBe(false)
    expect(result.error?.issues[0]?.path).toEqual(['threads'])
  })

  test('rejects the wrong primitive type', () => {
    const result = LlamaConfigRequestSchema.safeParse({
      ctx_size: '8192',
      flash_attn: 'yes'
    })
    expect(result.success).toBe(false)
    expect(result.error?.issues.map((i) => i.path[0]).sort()).toEqual([
      'ctx_size',
      'flash_attn'
    ])
  })
})

describe('buildLlamaConfigPayload', () => {
  test('drops every empty numeric field and every false boolean', () => {
    expect(buildLlamaConfigPayload(emptyForm)).toEqual({
      hf_model: '',
      ctx_size: 0,
      model: ''
    })
  })

  test('always sends hf_model, trimmed, even when blank', () => {
    expect(
      buildLlamaConfigPayload({ ...emptyForm, hf_model: '  a/b  ' }).hf_model
    ).toBe('a/b')
    expect(
      buildLlamaConfigPayload({ ...emptyForm, hf_model: '   ' }).hf_model
    ).toBe('')
  })

  test('omits a negative context size', () => {
    const payload = buildLlamaConfigPayload({ ...emptyForm, ctx_size: -1 })
    expect('ctx_size' in payload).toBe(false)
  })

  test('keeps a context size of 0', () => {
    expect(
      buildLlamaConfigPayload({ ...emptyForm, ctx_size: 0 }).ctx_size
    ).toBe(0)
  })

  test('includes each optional numeric field once it is set', () => {
    expect(
      buildLlamaConfigPayload({
        ...emptyForm,
        ctx_size: 4096,
        threads: 8,
        threads_batch: 4,
        predict: -1,
        batch_size: 2048,
        ubatch_size: 512,
        gpu_layers: 0
      })
    ).toEqual({
      hf_model: '',
      ctx_size: 4096,
      threads: 8,
      threads_batch: 4,
      predict: -1,
      batch_size: 2048,
      ubatch_size: 512,
      gpu_layers: 0,
      model: ''
    })
  })

  test('includes the boolean flags only when they are true', () => {
    const payload = buildLlamaConfigPayload({
      ...emptyForm,
      flash_attn: true,
      mlock: true,
      no_mmap: true
    })
    expect(payload).toMatchObject({
      flash_attn: true,
      mlock: true,
      no_mmap: true
    })

    const off = buildLlamaConfigPayload(emptyForm)
    expect('flash_attn' in off).toBe(false)
    expect('mlock' in off).toBe(false)
    expect('no_mmap' in off).toBe(false)
  })

  test('sends a trimmed model path, including the empty string used to clear it', () => {
    expect(
      buildLlamaConfigPayload({ ...emptyForm, model: '  /models/a.gguf ' })
        .model
    ).toBe('/models/a.gguf')
    expect(buildLlamaConfigPayload({ ...emptyForm, model: '' }).model).toBe('')
  })

  test('skips the model key entirely when the value is undefined', () => {
    const payload = buildLlamaConfigPayload({
      ...emptyForm,
      model: undefined as unknown as string
    })
    expect('model' in payload).toBe(false)
  })

  test('produces a payload that satisfies the request schema', () => {
    const payload = buildLlamaConfigPayload({
      hf_model: ' unsloth/Qwen3-8B-GGUF ',
      ctx_size: 8192,
      threads: 8,
      threads_batch: '',
      predict: '',
      batch_size: 2048,
      ubatch_size: 512,
      flash_attn: true,
      mlock: false,
      no_mmap: false,
      gpu_layers: 99,
      model: ' /models/a.gguf '
    })
    const result = LlamaConfigRequestSchema.safeParse(payload)
    expect(result.success).toBe(true)
    expect(result.data).toEqual({
      hf_model: 'unsloth/Qwen3-8B-GGUF',
      ctx_size: 8192,
      threads: 8,
      batch_size: 2048,
      ubatch_size: 512,
      flash_attn: true,
      gpu_layers: 99,
      model: '/models/a.gguf'
    })
  })
})
