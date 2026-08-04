import { describe, expect, test } from 'vitest'
import { formatFileSize, isLocalPath, getDisplayValue } from './utils.ts'

describe('formatFileSize', () => {
  test('reports unknown size for undefined and zero', () => {
    expect(formatFileSize()).toBe('Unknown size')
    expect(formatFileSize(undefined)).toBe('Unknown size')
    expect(formatFileSize(0)).toBe('Unknown size')
  })

  test('keeps bytes below the first threshold', () => {
    expect(formatFileSize(1)).toBe('1.00 B')
    expect(formatFileSize(1023)).toBe('1023.00 B')
  })

  test('steps up one unit at a time', () => {
    expect(formatFileSize(1024)).toBe('1.00 KB')
    expect(formatFileSize(1536)).toBe('1.50 KB')
    expect(formatFileSize(1024 * 1024)).toBe('1.00 MB')
    expect(formatFileSize(5 * 1024 ** 3)).toBe('5.00 GB')
  })

  test('stops at terabytes instead of running off the unit list', () => {
    expect(formatFileSize(1024 ** 4)).toBe('1.00 TB')
    expect(formatFileSize(4096 * 1024 ** 4)).toBe('4096.00 TB')
  })
})

describe('isLocalPath', () => {
  test('returns false for an empty string', () => {
    expect(isLocalPath('')).toBe(false)
  })

  test.each([
    ['/models/a.gguf'],
    ['./a.gguf'],
    ['../models/a.gguf'],
    ['models\\a.gguf'],
    ['C:\\models\\a.gguf']
  ])('recognises %s as a local path', (value) => {
    expect(isLocalPath(value)).toBe(true)
  })

  test.each([['unsloth/Qwen3-8B-GGUF'], ['qwen3:8b'], ['a.gguf']])(
    'does not treat %s as a local path',
    (value) => {
      expect(isLocalPath(value)).toBe(false)
    }
  )
})

describe('getDisplayValue', () => {
  test('returns an empty string for an empty value', () => {
    expect(getDisplayValue('')).toBe('')
  })

  test('reduces a local path to its filename', () => {
    expect(getDisplayValue('/models/Qwen3-8B-Q8_0.gguf')).toBe(
      'Qwen3-8B-Q8_0.gguf'
    )
    expect(getDisplayValue('C:\\models\\Qwen3-8B-Q8_0.gguf')).toBe(
      'Qwen3-8B-Q8_0.gguf'
    )
  })

  test('keeps a HuggingFace repo id untouched', () => {
    expect(getDisplayValue('unsloth/Qwen3-8B-GGUF')).toBe(
      'unsloth/Qwen3-8B-GGUF'
    )
  })

  test('falls back to the original value when the path has no last segment', () => {
    expect(getDisplayValue('/models/')).toBe('/models/')
  })
})
