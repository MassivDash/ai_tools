import type { ModelNote } from '@types'

export interface ModelInfo {
  name: string
  path?: string
  hf_format?: string
  legacy_hf_format?: string
}

export function normalizeModelName(name: string): {
  base: string
  quant: string | null
  owner: string | null
} {
  if (!name) return { base: '', quant: null, owner: null }

  let clean = name.toLowerCase()
  let owner: string | null = null

  // 1. If it's a path, get only the last component (filename), and remember
  // a candidate owner. The HF hub cache lays files out as
  // .../models--<owner>--<repo>/snapshots/<hash>/<file>, so the immediate
  // parent directory is a content hash, not the owner - check for that
  // "models--" segment first. Otherwise, treat the parent as an owner only
  // for a plain two-segment "owner/repo" id (e.g. hf_format-style strings) -
  // a real multi-segment filesystem path's immediate parent is just a
  // directory name (e.g. "models", "llama.cpp", "storageA"), never a real
  // owner, and treating it as one causes false owner mismatches.
  const pathSegments = clean.split(/[/\\]/)
  const isSimpleOwnerRepoPair = pathSegments.length === 2
  const hfCacheSegment = pathSegments.find((segment) =>
    segment.startsWith('models--')
  )
  if (hfCacheSegment) {
    const hfCacheParts = hfCacheSegment.split('--')
    if (hfCacheParts[1]) {
      owner = hfCacheParts[1]
    }
  }
  const lastSegment = pathSegments.pop()
  if (lastSegment) {
    clean = lastSegment
    if (!owner && isSimpleOwnerRepoPair && pathSegments[0]) {
      owner = pathSegments[0]
    }
  }

  // 2. Remove extension like .gguf
  if (clean.endsWith('.gguf')) {
    clean = clean.substring(0, clean.length - 5)
  }

  // 3. Extract quantization pattern
  // Check for typical quant patterns: e.g. -q4_k_m, _q4_k_m, .q8_0, :q8_0, etc.
  const quantRegex =
    /[-_.:](q[0-9]_[a-z0-9_]+|iq[0-9]_[a-z0-9_]+|q[0-9]_[0-9]|[qf][0-9]+|bf16)/i
  const match = clean.match(quantRegex)
  let quant: string | null = null
  if (match) {
    quant = match[1].toLowerCase().replace(/_/g, '') // e.g. q4km, q80, f16
    // Remove the matched quant from the base name
    clean = clean.replace(match[0], '')
  }

  // 4. Clean up owner prefix from hf_format if present (e.g. "unsloth/glm..." -> "glm...")
  // Only reached when step 1 above couldn't isolate a filename (e.g. a
  // trailing slash), so it also needs to capture the owner if not set yet.
  const slashIndex = clean.indexOf('/')
  if (slashIndex !== -1) {
    const rest = clean.substring(slashIndex + 1)
    if (rest.length >= 3 && !/^\d+$/.test(rest)) {
      if (!owner) owner = clean.substring(0, slashIndex)
      clean = rest
    }
  }

  // Remove common owner prefixes from filenames
  const knownOwners = [
    'unsloth',
    'ggml-org',
    'ggml',
    'speakleash',
    'massivdash',
    'fortytwo-network',
    'fortytwo',
    'maziyarpanahi',
    'mlabonne',
    'second-state',
    'secondstate',
    'mradermacher',
    'sci-fi-vy',
    'phil2sat',
    'google',
    'openai',
    'openai-community',
    'sentence-transformers',
    'mistralai',
    'qwen'
  ]
  for (const knownOwner of knownOwners) {
    if (clean.startsWith(knownOwner + '_') || clean.startsWith(knownOwner + '-')) {
      if (!owner) owner = knownOwner
      clean = clean.substring(knownOwner.length + 1)
    }
  }

  // Remove common suffix words
  const suffixesToRemove = [
    '-gguf',
    '_gguf',
    '-instruct',
    '_instruct',
    '-it',
    '_it',
    '-chat',
    '_chat',
    '-abliterated',
    '_abliterated',
    '-finetuned',
    '_finetuned',
    '-heretic',
    '_heretic',
    '-v3.0',
    '-v3',
    '-v2.0',
    '-v2',
    '-v1.0',
    '-v1',
    '-ud',
    '_ud'
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

  return { base: clean, quant, owner }
}

export function modelsMatch(
  model: ModelInfo,
  noteModelName: string,
  noteModelPath?: string
): boolean {
  if (!noteModelName) return false

  // 1. Direct exact matches
  if (model.name === noteModelName) return true
  if (model.path && model.path === noteModelPath) return true
  if (model.hf_format && model.hf_format === noteModelName) return true
  if (model.legacy_hf_format && model.legacy_hf_format === noteModelName)
    return true

  // 2. Fuzzy normalized matching. (Matching on the note's path by bare
  // filename alone - regardless of directory - used to be an exact-tier
  // shortcut here, but GGUF repos commonly reuse the same generic filename
  // across unrelated models, so it's handled below instead, where the
  // owner-aware check can tell those apart.)
  const normModelName = normalizeModelName(model.name)
  const normModelPath = model.path ? normalizeModelName(model.path) : null
  const normModelHf = model.hf_format
    ? normalizeModelName(model.hf_format)
    : null
  const normModelLegacy = model.legacy_hf_format
    ? normalizeModelName(model.legacy_hf_format)
    : null

  const normNoteName = normalizeModelName(noteModelName)
  const normNotePath = noteModelPath ? normalizeModelName(noteModelPath) : null

  const normModels = [
    normModelName,
    normModelPath,
    normModelHf,
    normModelLegacy
  ].filter(Boolean) as { base: string; quant: string | null; owner: string | null }[]
  const normNotes = [normNoteName, normNotePath].filter(Boolean) as {
    base: string
    quant: string | null
    owner: string | null
  }[]

  // Different explicit owners (e.g. two different HF orgs/users) must never
  // fuzzy-match just because a repo/base name coincides - even if the one
  // representation that happens to line up on base+quant doesn't itself
  // carry owner info (e.g. a bare filename has no owner, but the same
  // model's hf_format or HF-cache path does). Compare on whichever
  // representation of each side has an owner, not per-pairing.
  const modelOwner = normModels.map((m) => m.owner).find(Boolean) ?? null
  const noteOwner = normNotes.map((n) => n.owner).find(Boolean) ?? null
  if (modelOwner && noteOwner && modelOwner !== noteOwner) {
    return false
  }

  for (const m of normModels) {
    for (const n of normNotes) {
      // Must match quantizations if both have one
      if (m.quant && n.quant && m.quant !== n.quant) {
        continue
      }
      // If one base name contains the other, we consider it a fuzzy match -
      // unless both sides have a known owner, in which case containment is
      // too loose: two different repos from the same owner routinely differ
      // only by a size suffix (e.g. "Foo" vs "Foo-12B"), which containment
      // would wrongly treat as the same model. With both owners already
      // confirmed equal (checked above), require the repo name itself to
      // match exactly instead.
      if (m.base && n.base) {
        if (m.base.length < 3 || n.base.length < 3) {
          if (m.base === n.base) return true
        } else if (m.owner && n.owner) {
          if (m.base === n.base) return true
        } else if (m.base.includes(n.base) || n.base.includes(m.base)) {
          return true
        }
      }
    }
  }

  return false
}

export function findNoteForModel(
  model: ModelInfo,
  notes: Map<string, ModelNote> | ModelNote[]
): ModelNote | null {
  const notesList = notes instanceof Map ? Array.from(notes.values()) : notes
  for (const note of notesList) {
    if (modelsMatch(model, note.model_name, note.model_path)) {
      return note
    }
  }
  return null
}
