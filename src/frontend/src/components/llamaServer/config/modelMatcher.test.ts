import { describe, expect, test } from 'vitest'
import {
  normalizeModelName,
  modelsMatch,
  findNoteForModel,
  type ModelInfo
} from './modelMatcher.ts'
import type { ModelNote } from '@types'

const note = (
  over: Partial<ModelNote> & { model_name: string }
): ModelNote => ({
  platform: 'llama',
  is_favorite: false,
  is_default: false,
  tags: [],
  ...over
})

describe('normalizeModelName', () => {
  test('returns an empty result for an empty name', () => {
    expect(normalizeModelName('')).toEqual({
      base: '',
      quant: null,
      owner: null
    })
  })

  test('keeps only the filename of a posix path, drops .gguf and extracts quant, and does not treat the generic parent directory as an owner', () => {
    expect(normalizeModelName('/models/Qwen3-4B-Instruct-Q4_K_M.gguf')).toEqual(
      {
        base: 'qwen3-4b',
        quant: 'q4km',
        owner: null
      }
    )
  })

  test('keeps only the filename of a windows path, and does not treat the generic parent directory as an owner', () => {
    expect(normalizeModelName('C:\\models\\phi-4-Q5_K_S.gguf')).toEqual({
      base: 'phi-4',
      quant: 'q5ks',
      owner: null
    })
  })

  test('strips the owner segment of an HF repo id', () => {
    expect(normalizeModelName('unsloth/GLM-4.5-Air-GGUF')).toEqual({
      base: 'glm-4.5-air',
      quant: null,
      owner: 'unsloth'
    })
  })

  test('strips a known owner prefix joined with an underscore', () => {
    expect(normalizeModelName('unsloth_Qwen3-8B-Q8_0.gguf')).toEqual({
      base: 'qwen3-8b',
      quant: 'q80',
      owner: 'unsloth'
    })
  })

  test('strips a known owner prefix joined with a dash', () => {
    expect(normalizeModelName('google-gemma-3-4b.gguf')).toEqual({
      base: 'gemma-3-4b',
      quant: null,
      owner: 'google'
    })
  })

  test('does not strip a known owner that is only a prefix of a longer word', () => {
    // 'qwen' is a known owner, but 'qwen3-...' must keep its name intact
    expect(normalizeModelName('Qwen3-8B').base).toBe('qwen3-8b')
  })

  test('strips owner both from the directory and from the filename', () => {
    expect(normalizeModelName('ggml-org/ggml-org-whisper-base.gguf')).toEqual({
      base: 'whisper-base',
      quant: null,
      owner: 'ggml-org'
    })
  })

  test.each([
    ['model-f16.gguf', 'f16'],
    ['model-BF16.gguf', 'bf16'],
    ['model.IQ4_XS.gguf', 'iq4xs'],
    ['model:Q8_0', 'q80']
  ])('extracts quantization %s -> %s', (name, quant) => {
    expect(normalizeModelName(name)).toEqual({
      base: 'model',
      quant,
      owner: null
    })
  })

  test('returns a null quant when no quantization marker is present', () => {
    expect(normalizeModelName('gemma-3').quant).toBeNull()
  })

  test('removes stacked suffixes until none are left', () => {
    // -gguf then -it are both removed by repeated passes of the suffix loop
    expect(normalizeModelName('Gemma-3-4B-it-GGUF').base).toBe('gemma-3-4b')
    expect(normalizeModelName('llama-3-v2.0-instruct').base).toBe('llama-3')
    expect(normalizeModelName('Model-Chat-Abliterated-v1').base).toBe('model')
  })

  test('trims leftover separators from the edges', () => {
    expect(normalizeModelName('mistral-7b-').base).toBe('mistral-7b')
    expect(normalizeModelName('_-.phi-3.-_').base).toBe('phi-3')
  })

  test('normalizes to an empty base when the name is only a suffix word', () => {
    expect(normalizeModelName('-instruct')).toEqual({
      base: '',
      quant: null,
      owner: null
    })
  })

  test('keeps the version fragment that is not a stripped suffix', () => {
    expect(normalizeModelName('nomic-embed-text-v1.5.Q8_0.gguf')).toEqual({
      base: 'nomic-embed-text-v1.5',
      quant: 'q80',
      owner: null
    })
  })

  test('extracts the owner from an HF hub cache "models--owner--repo" directory, not the snapshot hash', () => {
    const path =
      '/home/user/.cache/huggingface/hub/models--professorf--gemma-4-12B-it-gguf/snapshots/3cdb2856/gemma-4-12B-it-q6_k.gguf'
    expect(normalizeModelName(path).owner).toBe('professorf')
  })

  test('drops the leading segment when a trailing slash defeats the filename split', () => {
    // pop() yields '' for a trailing slash, so the raw value is kept and the
    // first-slash owner strip applies instead
    expect(normalizeModelName('a/bcd/').base).toBe('bcd/')
  })

  test('keeps the whole value when the remainder after the first slash is too short', () => {
    expect(normalizeModelName('a/b/').base).toBe('a/b/')
    expect(normalizeModelName('/').base).toBe('/')
  })

  test('strips the leading segment of a numeric directory name', () => {
    expect(normalizeModelName('a/1234/').base).toBe('1234/')
  })
})

describe('modelsMatch', () => {
  test('rejects an empty note model name', () => {
    expect(modelsMatch({ name: 'qwen3-8b' }, '')).toBe(false)
  })

  test('matches an identical name', () => {
    expect(modelsMatch({ name: 'qwen3-8b' }, 'qwen3-8b')).toBe(true)
  })

  test('matches an identical path even when the names differ', () => {
    expect(
      modelsMatch(
        { name: 'model-a', path: '/models/x.gguf' },
        'model-b',
        '/models/x.gguf'
      )
    ).toBe(true)
  })

  test('matches on hf_format', () => {
    expect(
      modelsMatch(
        { name: 'x.gguf', hf_format: 'unsloth/Qwen3-8B-GGUF:Q8_0' },
        'unsloth/Qwen3-8B-GGUF:Q8_0'
      )
    ).toBe(true)
  })

  test('matches on legacy_hf_format', () => {
    expect(
      modelsMatch(
        {
          name: 'x.gguf',
          hf_format: 'unsloth/Qwen3-8B-GGUF:Q8_0',
          legacy_hf_format: 'unsloth/Qwen3-8B-GGUF'
        },
        'unsloth/Qwen3-8B-GGUF'
      )
    ).toBe(true)
  })

  test('matches when only the filenames of the two paths are equal', () => {
    expect(
      modelsMatch(
        { name: 'a', path: '/srv/models/Qwen3-8B-Q8_0.gguf' },
        'b',
        'C:\\downloads\\Qwen3-8B-Q8_0.gguf'
      )
    ).toBe(true)
  })

  test('does not match on paths whose filenames differ and whose bases differ', () => {
    expect(
      modelsMatch(
        { name: 'alpha', path: '/srv/models/alpha.gguf' },
        'beta',
        '/srv/models/beta.gguf'
      )
    ).toBe(false)
  })

  test('skips the filename comparison when a path has no filename component', () => {
    expect(
      modelsMatch({ name: 'alpha', path: '/' }, 'beta', '/srv/beta.gguf')
    ).toBe(false)
  })

  test('fuzzy-matches when the model base contains the note base', () => {
    expect(modelsMatch({ name: 'qwen3-8b-instruct.gguf' }, 'Qwen3-8B')).toBe(
      true
    )
  })

  test('fuzzy-matches when the note base contains the model base', () => {
    expect(modelsMatch({ name: 'gemma-3' }, 'gemma-3-4b-it')).toBe(true)
  })

  test('fuzzy-matches through the model path when the name does not match', () => {
    expect(
      modelsMatch(
        { name: 'served-model', path: '/models/Qwen3-4B-Q4_K_M.gguf' },
        'qwen3-4b'
      )
    ).toBe(true)
  })

  test('fuzzy-matches through the note path when the note name does not match', () => {
    expect(
      modelsMatch(
        { name: 'Qwen3-4B-Instruct-Q4_K_M.gguf' },
        'friendly label',
        '/models/qwen3-4b.gguf'
      )
    ).toBe(true)
  })

  test('fuzzy-matches through hf_format and legacy_hf_format', () => {
    expect(
      modelsMatch(
        {
          name: 'served',
          hf_format: 'unsloth/GLM-4.5-Air-GGUF:Q4_K_M',
          legacy_hf_format: 'unsloth/GLM-4.5-Air-GGUF'
        },
        'glm-4.5-air'
      )
    ).toBe(true)
  })

  test('refuses a fuzzy match when both sides declare a different quantization', () => {
    expect(
      modelsMatch({ name: 'Qwen3-8B-Q4_K_M.gguf' }, 'Qwen3-8B-Q8_0.gguf')
    ).toBe(false)
  })

  test('allows a fuzzy match when only one side declares a quantization', () => {
    expect(modelsMatch({ name: 'Qwen3-8B-Q4_K_M.gguf' }, 'Qwen3-8B')).toBe(true)
  })

  test('requires exact equality for bases shorter than three characters', () => {
    expect(modelsMatch({ name: 'AB' }, 'ab')).toBe(true)
    expect(modelsMatch({ name: 'AB' }, 'cd')).toBe(false)
    // 'ab' is a substring of 'abc' but the short base rule forbids the match
    expect(modelsMatch({ name: 'AB' }, 'abc')).toBe(false)
  })

  test('does not match when the model name normalizes to an empty base', () => {
    expect(modelsMatch({ name: '' }, 'qwen3-8b')).toBe(false)
  })

  test('does not match when the note name normalizes to an empty base', () => {
    expect(modelsMatch({ name: 'qwen3-8b' }, '-instruct')).toBe(false)
  })

  test('does not fuzzy-match repos with the same name but different owners', () => {
    expect(
      modelsMatch({ name: 'massivDash/gemma-4' }, 'professor/gemma-4')
    ).toBe(false)
  })

  test('still fuzzy-matches when only one side declares an owner', () => {
    expect(modelsMatch({ name: 'massivDash/gemma-4' }, 'gemma-4')).toBe(true)
  })

  test('does not fuzzy-match two different HF repos that reuse the same converted filename', () => {
    // Real-world case: two unrelated HF orgs both re-uploaded a GGUF conversion
    // of "gemma-4-12B-it" with the same quant, so the bare filenames collide.
    // Only the directory structure (models--owner--repo) tells them apart.
    const massivdashPath =
      '/home/user/.cache/huggingface/hub/models--MassivDash--Gemma-4-RUST-CODER-12B/snapshots/b0ea4f15/gemma-4-12B-it.Q6_K.gguf'
    const professorfPath =
      '/home/user/.cache/huggingface/hub/models--professorf--gemma-4-12B-it-gguf/snapshots/3cdb2856/gemma-4-12B-it-q6_k.gguf'

    expect(
      modelsMatch(
        {
          name: 'gemma-4-12B-it-q6_k.gguf',
          path: professorfPath,
          hf_format: 'professorf/gemma-4-12B-it-gguf'
        },
        'MassivDash/Gemma-4-RUST-CODER-12B:Q6_K',
        massivdashPath
      )
    ).toBe(false)
  })

  test('still fuzzy-matches a model cached flatly under a generic local directory against its own hf_format-keyed note', () => {
    // A model downloaded via llama.cpp's own cache lands as a flat file
    // (no HF-cache "models--owner--repo" structure) - the parent directory
    // is just "llama.cpp"/"models"/etc, not a real owner, and must not be
    // treated as one or it would wrongly disagree with the real owner
    // ("unsloth") parsed from hf_format.
    expect(
      modelsMatch(
        {
          name: 'random-quant-name.gguf',
          path: '/home/user/.cache/llama.cpp/random-quant-name.gguf',
          hf_format: 'unsloth/Foo-GGUF:Q4_K_M'
        },
        // Not an exact string match against hf_format, so this must be
        // resolved through the fuzzy/owner-aware tier.
        'unsloth/Foo-GGUF'
      )
    ).toBe(true)
  })

  test('still fuzzy-matches the same model recorded under two differently-named generic local directories', () => {
    expect(
      modelsMatch(
        { name: 'renamed.gguf', path: '/mnt/storageA/qwen3-8b-q4_k_m.gguf' },
        'original-name',
        '/mnt/storageB/qwen3-8b-Q4_K_M.gguf'
      )
    ).toBe(true)
  })

  test('does not fuzzy-match two different-sized models from the same owner whose repo name is a prefix of the other', () => {
    // Real MassivDash data: a note already exists for the 12B model
    // (repo "Gemma-4-RUST-CODER-12B"), and the unrelated 5B model (repo
    // "Gemma-4-Rust-Coder") happens to share the owner and quant. The size
    // suffix "-12B" is the only difference, so naive substring containment
    // would treat them as the same model - they are not.
    expect(
      modelsMatch(
        {
          name: 'gemma-4-e2b-it.Q8_0.gguf',
          path: '/home/user/.cache/huggingface/hub/models--MassivDash--Gemma-4-Rust-Coder/snapshots/642/gemma-4-e2b-it.Q8_0.gguf',
          hf_format: 'MassivDash/Gemma-4-Rust-Coder:Q8_0'
        },
        'MassivDash/Gemma-4-RUST-CODER-12B:Q8_0'
      )
    ).toBe(false)
  })
})

describe('findNoteForModel', () => {
  const model: ModelInfo = {
    name: 'Qwen3-8B-Instruct-Q8_0.gguf',
    path: '/models/Qwen3-8B-Instruct-Q8_0.gguf'
  }

  test('returns the first matching note from an array', () => {
    const notes = [
      note({ model_name: 'gemma-3-4b' }),
      note({ model_name: 'Qwen3-8B', notes: 'the one' }),
      note({ model_name: 'qwen3-8b', notes: 'later duplicate' })
    ]
    expect(findNoteForModel(model, notes)?.notes).toBe('the one')
  })

  test('returns the matching note from a Map keyed by name', () => {
    const notes = new Map<string, ModelNote>([
      ['a', note({ model_name: 'llama-3-8b' })],
      ['b', note({ model_name: 'qwen3-8b', notes: 'from map' })]
    ])
    expect(findNoteForModel(model, notes)?.notes).toBe('from map')
  })

  test('matches by note model_path when the note name is unrelated', () => {
    const notes = [
      note({
        model_name: 'my favourite',
        model_path: '/elsewhere/Qwen3-8B-Instruct-Q8_0.gguf',
        notes: 'by path'
      })
    ]
    expect(findNoteForModel(model, notes)?.notes).toBe('by path')
  })

  test('returns null when nothing matches', () => {
    expect(findNoteForModel(model, [note({ model_name: 'gemma-3-4b' })])).toBe(
      null
    )
    expect(findNoteForModel(model, new Map())).toBe(null)
  })
})
