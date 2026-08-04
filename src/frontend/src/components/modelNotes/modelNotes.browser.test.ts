/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import ModelNotes from './modelNotes.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { Component } from 'svelte'
import type { ModelNote } from '@types'

vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn(),
    delete: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
  delete: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

// Two real, distinct MassivDash repos whose converted GGUF filenames both
// normalize to "gemma-4-rust-coder" + the same quant - the actual data that
// triggered the same-owner false-favorite bug.
const twelveB = {
  name: 'gemma-4-12B-it.Q8_0.gguf',
  path: '/home/user/.cache/huggingface/hub/models--MassivDash--Gemma-4-RUST-CODER-12B/snapshots/b0ea4f15/gemma-4-12B-it.Q8_0.gguf',
  size: 12000000000,
  hf_format: 'MassivDash/Gemma-4-RUST-CODER-12B:Q8_0'
}

const fiveB = {
  name: 'gemma-4-e2b-it.Q8_0.gguf',
  path: '/home/user/.cache/huggingface/hub/models--MassivDash--Gemma-4-Rust-Coder/snapshots/64240483/gemma-4-e2b-it.Q8_0.gguf',
  size: 5000000000,
  hf_format: 'MassivDash/Gemma-4-Rust-Coder:Q8_0'
}

const mockGets = (
  notes: ModelNote[] = [],
  models: unknown[] = [twelveB, fiveB]
) => {
  const responses: Record<string, unknown> = {
    'llama-server/models': { local_models: models },
    'chromadb/models': { models: [] },
    'model-notes': { notes }
  }
  mockedAxios.get.mockImplementation((url: string) => {
    if (!(url in responses)) {
      return Promise.reject(new Error(`Unexpected URL: ${url}`))
    }
    return Promise.resolve({ data: responses[url] })
  })
}

const renderPage = async (
  expectedTexts: string[] = [twelveB.hf_format, fiveB.hf_format]
) => {
  const rendered = render(ModelNotes as Component)
  await waitFor(() => {
    for (const text of expectedTexts) {
      expect(screen.getByText(text)).toBeTruthy()
    }
  })
  return rendered
}

const cardFor = (hfFormat: string) =>
  screen.getByText(hfFormat).closest('.model-card') as HTMLElement

test('favoriting the 12B model does not also favorite the similarly-named 5B model', async () => {
  mockGets()
  mockedAxios.post.mockImplementation((_url: string, body: unknown) =>
    Promise.resolve({
      data: {
        note: {
          platform: 'llama',
          is_favorite: true,
          is_default: false,
          tags: [],
          ...(body as object)
        }
      }
    })
  )

  await renderPage()

  const twelveBCard = cardFor(twelveB.hf_format)
  await fireEvent.click(
    within(twelveBCard).getByTitle('Add to favorites')
  )

  // The note must be keyed by the model's exact file path, not the guessed
  // hf_format string - that's what keeps it from colliding with the 5B repo.
  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'model-notes',
      expect.objectContaining({ model_name: twelveB.path })
    )
  })

  await waitFor(() => {
    expect(
      within(cardFor(twelveB.hf_format)).getByTitle('Remove from favorites')
    ).toBeTruthy()
  })

  // The 5B card must remain untouched.
  expect(
    within(cardFor(fiveB.hf_format)).getByTitle('Add to favorites')
  ).toBeTruthy()
})

test('notes/tags added to the 12B MassivDash model do not show up on the unrelated 5B model', async () => {
  // Real report: editing notes on "Gemma-4-RUST-CODER-12B" also showed those
  // notes on "Gemma-4-Rust-Coder" (the 5B model) - same owner, same quant,
  // and "Gemma-4-Rust-Coder" is a literal prefix of the 12B repo's name.
  const twelveBNote: ModelNote = {
    platform: 'llama',
    model_name: twelveB.hf_format,
    model_path: twelveB.path,
    is_favorite: false,
    is_default: false,
    tags: ['Q8'],
    notes: 'slow'
  }

  mockGets([twelveBNote])
  await renderPage()

  expect(within(cardFor(twelveB.hf_format)).getByText('slow')).toBeTruthy()
  expect(within(cardFor(twelveB.hf_format)).getByText('Q8')).toBeTruthy()

  const fiveBCard = cardFor(fiveB.hf_format)
  expect(within(fiveBCard).queryByText('slow')).toBeNull()
  expect(within(fiveBCard).queryByText('Q8')).toBeNull()
})

test('unfavoriting a model with a pre-existing legacy-keyed note updates the card', async () => {
  // Real case: an older note was saved under the legacy llama.cpp-cache-style
  // filename key, which differs from the model's own exact path.
  const legacyModel = {
    name: 'DeepSeek-R1-0528-Qwen3-8B-UD-Q6_K_XL.gguf',
    path: '/home/user/.cache/llama.cpp/unsloth_DeepSeek-R1-0528-Qwen3-8B-GGUF_DeepSeek-R1-0528-Qwen3-8B-UD-Q6_K_XL.gguf',
    hf_format: 'unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL'
  }
  const existingNote: ModelNote = {
    platform: 'llama',
    model_name:
      'unsloth_DeepSeek-R1-0528-Qwen3-8B-GGUF_DeepSeek-R1-0528-Qwen3-8B-UD-Q6_K_XL.gguf',
    model_path: legacyModel.path,
    is_favorite: true,
    is_default: false,
    tags: []
  }

  mockGets([existingNote], [legacyModel])
  mockedAxios.post.mockImplementation((_url: string, body: unknown) =>
    Promise.resolve({ data: { note: { ...existingNote, ...(body as object) } } })
  )

  await renderPage([legacyModel.hf_format])

  const card = cardFor(legacyModel.hf_format)
  expect(within(card).getByTitle('Remove from favorites')).toBeTruthy()

  await fireEvent.click(within(card).getByTitle('Remove from favorites'))

  // The backend upsert keeps the note under its original (legacy) key, not
  // the model's path - the UI must still reflect the new is_favorite value
  // for that same note, not show a stale cached copy.
  await waitFor(() => {
    expect(
      within(cardFor(legacyModel.hf_format)).getByTitle('Add to favorites')
    ).toBeTruthy()
  })
})

test('two different repos that share the same bare filename do not share a favorite', async () => {
  // Real-world case: many GGUF uploaders reuse the exact same generic
  // filename convention (e.g. "model-Q4_K_M.gguf") across unrelated repos.
  const repoA = {
    name: 'model-Q4_K_M.gguf',
    path: '/home/user/.cache/huggingface/hub/models--RepoA--Thing/snapshots/aaa/model-Q4_K_M.gguf',
    hf_format: 'RepoA/Thing:Q4_K_M'
  }
  const repoB = {
    name: 'model-Q4_K_M.gguf',
    path: '/home/user/.cache/huggingface/hub/models--RepoB--Other/snapshots/bbb/model-Q4_K_M.gguf',
    hf_format: 'RepoB/Other:Q4_K_M'
  }
  // repoA was favorited in an earlier session - its note is keyed by its
  // exact path (today's path-first keying), which differs from the shared
  // bare filename.
  const repoANote: ModelNote = {
    platform: 'llama',
    model_name: repoA.path,
    model_path: repoA.path,
    is_favorite: true,
    is_default: false,
    tags: []
  }

  mockGets([repoANote], [repoA, repoB])
  await renderPage([repoA.hf_format, repoB.hf_format])

  expect(
    within(cardFor(repoA.hf_format)).getByTitle('Remove from favorites')
  ).toBeTruthy()
  // repoB must NOT inherit repoA's favorite just because they share a
  // filename - only its own exact path/hf_format identify it.
  expect(
    within(cardFor(repoB.hf_format)).getByTitle('Add to favorites')
  ).toBeTruthy()
})

test('deleting a note asks for confirmation once, even though several legacy keys are tried', async () => {
  const model = {
    name: 'model-Q4_K_M.gguf',
    path: '/home/user/.cache/huggingface/hub/models--RepoA--Thing/snapshots/aaa/model-Q4_K_M.gguf',
    hf_format: 'RepoA/Thing:Q4_K_M'
  }
  const existingNote: ModelNote = {
    platform: 'llama',
    model_name: model.path,
    model_path: model.path,
    is_favorite: true,
    is_default: false,
    tags: []
  }

  mockGets([existingNote], [model])
  const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true)
  // Only the path-keyed delete matches a real row; the hf_format/name
  // attempts are expected 404s for keys nothing was ever saved under.
  mockedAxios.delete.mockImplementation((url: string) => {
    if (url === `model-notes/llama/${encodeURIComponent(model.path)}`) {
      return Promise.resolve({ data: { success: true } })
    }
    return Promise.reject({ response: { status: 404, data: { error: 'Model note not found' } } })
  })

  await renderPage([model.hf_format])

  await fireEvent.click(
    within(cardFor(model.hf_format)).getByTitle('Delete notes')
  )

  await waitFor(() => {
    expect(mockedAxios.delete).toHaveBeenCalledWith(
      `model-notes/llama/${encodeURIComponent(model.path)}`
    )
  })

  expect(confirmSpy).toHaveBeenCalledTimes(1)
  // A harmless 404 on a stale legacy key must not surface as an error banner
  // once the real delete succeeded.
  expect(screen.queryByText(/not found/i)).toBeNull()
})

test('marking a brand-new note-less model as default persists its hf_format, not its raw file path', async () => {
  const model = {
    name: 'model-Q4_K_M.gguf',
    path: '/home/user/.cache/huggingface/hub/models--RepoA--Thing/snapshots/aaa/model-Q4_K_M.gguf',
    hf_format: 'RepoA/Thing:Q4_K_M'
  }

  mockGets([], [model])
  mockedAxios.post.mockImplementation((_url: string, body: unknown) =>
    Promise.resolve({
      data: {
        note: { platform: 'llama', is_favorite: false, tags: [], ...(body as object) }
      }
    })
  )

  await renderPage([model.hf_format])

  await fireEvent.click(
    within(cardFor(model.hf_format)).getByTitle('Edit notes')
  )
  await waitFor(() => {
    expect(screen.getByText('Set as Default Llama Model')).toBeTruthy()
  })
  await fireEvent.click(screen.getByText('Set as Default Llama Model'))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'model-notes',
      expect.objectContaining({ model_name: model.hf_format })
    )
  })
})
