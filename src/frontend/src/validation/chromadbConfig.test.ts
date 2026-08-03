import { describe, expect, test } from 'vitest'
import {
  ChromaDBConfigRequestSchema,
  buildChromaDBConfigPayload
} from './chromadbConfig.ts'

describe('ChromaDBConfigRequestSchema', () => {
  test('accepts a minimal payload and trims the embedding model', () => {
    const result = ChromaDBConfigRequestSchema.safeParse({
      embedding_model: '  all-MiniLM-L6-v2  '
    })
    expect(result.success).toBe(true)
    expect(result.data).toEqual({ embedding_model: 'all-MiniLM-L6-v2' })
  })

  test('accepts every optional field', () => {
    const result = ChromaDBConfigRequestSchema.safeParse({
      embedding_model: 'nomic-embed-text',
      query_model: '  nomic-embed-text  ',
      chunk_size: 1000,
      chunk_overlap: 0
    })
    expect(result.success).toBe(true)
    expect(result.data).toEqual({
      embedding_model: 'nomic-embed-text',
      query_model: 'nomic-embed-text',
      chunk_size: 1000,
      chunk_overlap: 0
    })
  })

  test('rejects a missing embedding model', () => {
    const result = ChromaDBConfigRequestSchema.safeParse({})
    expect(result.success).toBe(false)
    expect(result.error?.issues[0]?.path).toEqual(['embedding_model'])
    expect(result.error?.issues[0]?.code).toBe('invalid_type')
  })

  test.each([[''], ['   ']])(
    'rejects the blank embedding model %j with the custom message',
    (value) => {
      const result = ChromaDBConfigRequestSchema.safeParse({
        embedding_model: value
      })
      expect(result.success).toBe(false)
      expect(result.error?.issues).toHaveLength(1)
      expect(result.error?.issues[0]?.message).toBe(
        'Embedding model cannot be empty'
      )
      expect(result.error?.issues[0]?.path).toEqual(['embedding_model'])
    }
  )

  test('rejects a blank query model', () => {
    const result = ChromaDBConfigRequestSchema.safeParse({
      embedding_model: 'ok',
      query_model: '  '
    })
    expect(result.success).toBe(false)
    expect(result.error?.issues[0]?.message).toBe('Query model cannot be empty')
    expect(result.error?.issues[0]?.path).toEqual(['query_model'])
  })

  test('rejects a non-integer chunk size', () => {
    const result = ChromaDBConfigRequestSchema.safeParse({
      embedding_model: 'ok',
      chunk_size: 1.5
    })
    expect(result.success).toBe(false)
    expect(result.error?.issues[0]?.path).toEqual(['chunk_size'])
    expect(result.error?.issues[0]?.code).toBe('invalid_type')
  })

  test('enforces the chunk size lower bound of 1', () => {
    expect(
      ChromaDBConfigRequestSchema.safeParse({
        embedding_model: 'ok',
        chunk_size: 0
      }).success
    ).toBe(false)
    expect(
      ChromaDBConfigRequestSchema.safeParse({
        embedding_model: 'ok',
        chunk_size: 1
      }).success
    ).toBe(true)
  })

  test('enforces the chunk overlap lower bound of 0', () => {
    expect(
      ChromaDBConfigRequestSchema.safeParse({
        embedding_model: 'ok',
        chunk_overlap: -1
      }).success
    ).toBe(false)
    expect(
      ChromaDBConfigRequestSchema.safeParse({
        embedding_model: 'ok',
        chunk_overlap: 0
      }).success
    ).toBe(true)
  })
})

describe('buildChromaDBConfigPayload', () => {
  test('includes the trimmed embedding model', () => {
    expect(
      buildChromaDBConfigPayload({ embedding_model: '  all-MiniLM-L6-v2 ' })
    ).toEqual({ embedding_model: 'all-MiniLM-L6-v2' })
  })

  test('omits an embedding model that is blank after trimming', () => {
    expect(buildChromaDBConfigPayload({ embedding_model: '   ' })).toEqual({})
  })

  test('omits the query model when it is not provided', () => {
    const payload = buildChromaDBConfigPayload({ embedding_model: 'a' })
    expect('query_model' in payload).toBe(false)
  })

  test('omits the query model when it is provided but blank', () => {
    const payload = buildChromaDBConfigPayload({
      embedding_model: 'a',
      query_model: '  '
    })
    expect('query_model' in payload).toBe(false)
  })

  test('includes the trimmed query model when provided', () => {
    expect(
      buildChromaDBConfigPayload({
        embedding_model: 'a',
        query_model: ' b '
      })
    ).toEqual({ embedding_model: 'a', query_model: 'b' })
  })

  test('keeps zero-valued chunk numbers instead of dropping them', () => {
    expect(
      buildChromaDBConfigPayload({
        embedding_model: 'a',
        chunk_size: 0,
        chunk_overlap: 0
      })
    ).toEqual({ embedding_model: 'a', chunk_size: 0, chunk_overlap: 0 })
  })

  test('omits chunk numbers left undefined', () => {
    const payload = buildChromaDBConfigPayload({
      embedding_model: 'a',
      chunk_size: undefined,
      chunk_overlap: undefined
    })
    expect(payload).toEqual({ embedding_model: 'a' })
  })

  test('produces a payload that satisfies the request schema', () => {
    const payload = buildChromaDBConfigPayload({
      embedding_model: ' nomic-embed-text ',
      query_model: ' nomic-embed-text ',
      chunk_size: 512,
      chunk_overlap: 64
    })
    expect(ChromaDBConfigRequestSchema.safeParse(payload).success).toBe(true)
  })
})
