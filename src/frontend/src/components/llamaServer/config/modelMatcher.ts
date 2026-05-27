import type { ModelNote } from '@types'

export interface ModelInfo {
  name: string
  path?: string
  hf_format?: string
  legacy_hf_format?: string
}

export function normalizeModelName(name: string): { base: string; quant: string | null } {
  if (!name) return { base: '', quant: null }

  let clean = name.toLowerCase()

  // 1. If it's a path, get only the last component (filename)
  clean = clean.split(/[/\\]/).pop() || clean

  // 2. Remove extension like .gguf
  if (clean.endsWith('.gguf')) {
    clean = clean.substring(0, clean.length - 5)
  }

  // 3. Extract quantization pattern
  // Check for typical quant patterns: e.g. -q4_k_m, _q4_k_m, .q8_0, :q8_0, etc.
  const quantRegex = /[-_.:](q[0-9]_[a-z0-9_]+|iq[0-9]_[a-z0-9_]+|q[0-9]_[0-9]|[qf][0-9]+|bf16)/i
  const match = clean.match(quantRegex)
  let quant: string | null = null
  if (match) {
    quant = match[1].toLowerCase().replace(/_/g, '') // e.g. q4km, q80, f16
    // Remove the matched quant from the base name
    clean = clean.replace(match[0], '')
  }

  // 4. Clean up owner prefix from hf_format if present (e.g. "unsloth/glm..." -> "glm...")
  const slashIndex = clean.indexOf('/')
  if (slashIndex !== -1) {
    const rest = clean.substring(slashIndex + 1)
    if (rest.length >= 3 && !/^\d+$/.test(rest)) {
      clean = rest
    }
  }

  // Remove common owner prefixes from filenames
  const knownOwners = [
    'unsloth', 'ggml-org', 'ggml', 'speakleash', 'massivdash', 'fortytwo-network', 'fortytwo',
    'maziyarpanahi', 'mlabonne', 'second-state', 'secondstate', 'mradermacher', 'sci-fi-vy',
    'phil2sat', 'google', 'openai', 'openai-community', 'sentence-transformers', 'mistralai', 'qwen'
  ]
  for (const owner of knownOwners) {
    if (clean.startsWith(owner + '_') || clean.startsWith(owner + '-')) {
      clean = clean.substring(owner.length + 1)
    }
  }

  // Remove common suffix words
  const suffixesToRemove = [
    '-gguf', '_gguf', '-instruct', '_instruct', '-it', '_it', '-chat', '_chat',
    '-abliterated', '_abliterated', '-finetuned', '_finetuned', '-heretic', '_heretic',
    '-v3.0', '-v3', '-v2.0', '-v2', '-v1.0', '-v1', '-ud', '_ud'
  ]
  
  let changed = true
  while (changed) {
    changed = false
    for (const suffix of suffixesToRemove) {
      if (clean.endsWith(suffix)) {
        clean = clean.substring(0, clean.length - suffix.length)
        changed = true
      }
    }
  }

  // Clean up any remaining trailing/leading dashes/underscores/dots
  clean = clean.replace(/^[-_.]+|[-_.]+$|\.gguf$/g, '').trim()

  return { base: clean, quant }
}

export function modelsMatch(model: ModelInfo, noteModelName: string, noteModelPath?: string): boolean {
  if (!noteModelName) return false

  // 1. Direct exact matches
  if (model.name === noteModelName) return true
  if (model.path && model.path === noteModelPath) return true
  if (model.hf_format && model.hf_format === noteModelName) return true
  if (model.legacy_hf_format && model.legacy_hf_format === noteModelName) return true

  // Check filename matching for note's model path
  if (noteModelPath && model.path) {
    const modelFilename = model.path.split(/[/\\]/).pop()
    const noteFilename = noteModelPath.split(/[/\\]/).pop()
    if (modelFilename && noteFilename && modelFilename === noteFilename) {
      return true
    }
  }

  // 2. Fuzzy normalized matching
  const normModelName = normalizeModelName(model.name)
  const normModelPath = model.path ? normalizeModelName(model.path) : null
  const normModelHf = model.hf_format ? normalizeModelName(model.hf_format) : null
  const normModelLegacy = model.legacy_hf_format ? normalizeModelName(model.legacy_hf_format) : null

  const normNoteName = normalizeModelName(noteModelName)
  const normNotePath = noteModelPath ? normalizeModelName(noteModelPath) : null

  const normModels = [normModelName, normModelPath, normModelHf, normModelLegacy].filter(Boolean) as { base: string; quant: string | null }[]
  const normNotes = [normNoteName, normNotePath].filter(Boolean) as { base: string; quant: string | null }[]

  for (const m of normModels) {
    for (const n of normNotes) {
      // Must match quantizations if both have one
      if (m.quant && n.quant && m.quant !== n.quant) {
        continue
      }
      // If one base name contains the other, we consider it a fuzzy match
      if (m.base && n.base) {
        if (m.base.length < 3 || n.base.length < 3) {
          if (m.base === n.base) return true
        } else if (m.base.includes(n.base) || n.base.includes(m.base)) {
          return true
        }
      }
    }
  }

  return false
}

export function findNoteForModel(model: ModelInfo, notes: Map<string, ModelNote> | ModelNote[]): ModelNote | null {
  const notesList = notes instanceof Map ? Array.from(notes.values()) : notes
  for (const note of notesList) {
    if (modelsMatch(model, note.model_name, note.model_path)) {
      return note
    }
  }
  return null
}
