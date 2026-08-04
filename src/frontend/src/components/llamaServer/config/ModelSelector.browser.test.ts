/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import ModelSelector from './ModelSelector.svelte'
import type { ModelInfo } from './modelMatcher'
import type { ModelNote } from '@types'

const models: ModelInfo[] = [
  {
    name: 'alpha-model.gguf',
    path: '/models/alpha-model.gguf',
    hf_format: 'org/alpha-model:Q4_K_M',
    size: 1536
  } as ModelInfo,
  {
    name: 'beta-model.gguf',
    path: '/models/beta-model.gguf'
  },
  {
    name: 'gamma-model.gguf'
  }
]

const note = (overrides: Partial<ModelNote>): ModelNote => ({
  platform: 'llama',
  model_name: 'alpha-model.gguf',
  is_favorite: false,
  is_default: false,
  tags: [],
  ...overrides
})

const baseProps = {
  loadingModels: false,
  localModels: models,
  modelNotes: new Map<string, ModelNote>(),
  newHfModel: '',
  newHfModelBackend: '',
  onSelect: () => {}
}

const itemFor = (label: string) =>
  screen.getByText(label).closest('button') as HTMLElement

test('shows the loading placeholder instead of the list while models load', () => {
  render(ModelSelector, {
    props: { ...baseProps, loadingModels: true }
  })

  expect(screen.getByText('Loading models...')).toBeTruthy()
  expect(
    screen.queryByPlaceholderText('Search models...')
  ).not.toBeInTheDocument()
})

test('shows the empty hint when there are no local models', () => {
  render(ModelSelector, {
    props: { ...baseProps, localModels: [] }
  })

  expect(
    screen.getByText(/No GGUF models found in ~\/\.cache\/llama\.cpp\//)
  ).toBeTruthy()
  expect(
    screen.queryByPlaceholderText('Search models...')
  ).not.toBeInTheDocument()
})

test('builds the subtext from hf_format, path and formatted size', () => {
  render(ModelSelector, { props: baseProps })

  // hf_format wins over path, size is appended
  expect(itemFor('alpha-model.gguf')).toHaveTextContent(
    'org/alpha-model:Q4_K_M • 1.50 KB'
  )
  // no hf_format -> falls back to the path, no size -> nothing appended
  const beta = itemFor('beta-model.gguf')
  expect(beta.querySelector('.item-subtext')).toHaveTextContent(
    '/models/beta-model.gguf'
  )
  expect(beta.querySelector('.item-subtext')).not.toHaveTextContent('KB')
  // neither hf_format nor path nor size -> empty subtext
  expect(
    itemFor('gamma-model.gguf').querySelector('.item-subtext')?.textContent
  ).toBe('')
})

test('decorates models with the favourite star, tags and a notes preview', () => {
  const notes = new Map<string, ModelNote>([
    [
      'alpha-model.gguf',
      note({
        model_name: 'alpha-model.gguf',
        is_favorite: true,
        tags: ['fast', 'coding'],
        notes: 'A'.repeat(150)
      })
    ],
    [
      'beta-model.gguf',
      note({ model_name: 'beta-model.gguf', notes: 'Short note' })
    ]
  ])

  const { container } = render(ModelSelector, {
    props: { ...baseProps, modelNotes: notes }
  })

  const alpha = itemFor('alpha-model.gguf')
  expect(alpha.querySelector('.favorite-icon')).toBeInTheDocument()
  expect(alpha.querySelector('.item-tags')).toHaveTextContent('fast')
  expect(alpha.querySelector('.item-tags')).toHaveTextContent('coding')
  // notes longer than 100 chars are truncated with an ellipsis
  expect(alpha.querySelector('.item-notes')?.textContent).toBe(
    'A'.repeat(100) + '...'
  )

  const beta = itemFor('beta-model.gguf')
  // not a favourite and no tags
  expect(beta.querySelector('.favorite-icon')).not.toBeInTheDocument()
  expect(beta.querySelector('.item-tags')).not.toBeInTheDocument()
  // short notes are shown verbatim
  expect(beta.querySelector('.item-notes')?.textContent).toBe('Short note')

  // only the favourite model gets a star
  expect(container.querySelectorAll('.favorite-icon')).toHaveLength(1)

  // a model without a matching note gets no notes block
  expect(
    itemFor('gamma-model.gguf').querySelector('.item-notes')
  ).not.toBeInTheDocument()
})

test('ignores notes belonging to a different model', () => {
  const notes = new Map<string, ModelNote>([
    [
      'something-else.gguf',
      note({
        model_name: 'something-else.gguf',
        is_favorite: true,
        tags: ['unrelated'],
        notes: 'unrelated note'
      })
    ]
  ])

  const { container } = render(ModelSelector, {
    props: { ...baseProps, modelNotes: notes }
  })

  expect(container.querySelector('.favorite-icon')).not.toBeInTheDocument()
  expect(screen.queryByText('unrelated')).not.toBeInTheDocument()
  expect(screen.queryByText('unrelated note')).not.toBeInTheDocument()
})

test('highlights the model whose path matches the backend value', () => {
  render(ModelSelector, {
    props: { ...baseProps, newHfModelBackend: '/models/beta-model.gguf' }
  })

  expect(itemFor('beta-model.gguf')).toHaveClass('selected')
  expect(itemFor('alpha-model.gguf')).not.toHaveClass('selected')
})

test('highlights the model whose hf_format matches the backend value', () => {
  render(ModelSelector, {
    props: { ...baseProps, newHfModelBackend: 'org/alpha-model:Q4_K_M' }
  })

  expect(itemFor('alpha-model.gguf')).toHaveClass('selected')
  expect(itemFor('beta-model.gguf')).not.toHaveClass('selected')
})

test('highlights nothing when neither the display nor the backend value matches', () => {
  render(ModelSelector, {
    props: {
      ...baseProps,
      newHfModel: 'unsloth/Some-Other-GGUF:Q6_K',
      newHfModelBackend: 'unsloth/Some-Other-GGUF:Q6_K'
    }
  })

  expect(document.querySelectorAll('.list-item.selected')).toHaveLength(0)
})

test('a path-less model matched by name is found but cannot be highlighted', () => {
  // getModelKey falls back to `${name}-${index}` while selectedKey falls back to
  // the bare name, so the two never line up for models without a path.
  render(ModelSelector, {
    props: {
      ...baseProps,
      localModels: [{ name: 'gamma-model.gguf' }],
      newHfModel: 'gamma-model.gguf'
    }
  })

  expect(itemFor('gamma-model.gguf')).not.toHaveClass('selected')
})

test('does not mark an entry without any identifier as selected', () => {
  const { container } = render(ModelSelector, {
    props: {
      ...baseProps,
      localModels: [{}] as unknown as ModelInfo[]
    }
  })

  expect(container.querySelectorAll('.list-item')).toHaveLength(1)
  expect(container.querySelector('.list-item')).not.toHaveClass('selected')
})

test('keeps path-less models with identical names as separate rows', () => {
  const { container } = render(ModelSelector, {
    props: {
      ...baseProps,
      localModels: [{ name: 'duplicate.gguf' }, { name: 'duplicate.gguf' }]
    }
  })

  expect(screen.getAllByText('duplicate.gguf')).toHaveLength(2)
  expect(container.querySelectorAll('.list-item')).toHaveLength(2)
})

test('keeps nameless models as separate rows using hf_format or the index', () => {
  const { container } = render(ModelSelector, {
    props: {
      ...baseProps,
      localModels: [
        { hf_format: 'org/repo:Q4' },
        { hf_format: 'org/repo:Q4' },
        {},
        {}
      ] as unknown as ModelInfo[]
    }
  })

  expect(container.querySelectorAll('.list-item')).toHaveLength(4)
})

test('forwards the clicked model to onSelect', async () => {
  const onSelect = vi.fn()
  render(ModelSelector, {
    props: { ...baseProps, onSelect }
  })

  await fireEvent.click(itemFor('alpha-model.gguf'))

  await waitFor(() => {
    expect(onSelect).toHaveBeenCalledTimes(1)
  })
  expect(onSelect).toHaveBeenCalledWith(
    expect.objectContaining({
      name: 'alpha-model.gguf',
      path: '/models/alpha-model.gguf',
      hf_format: 'org/alpha-model:Q4_K_M'
    })
  )
})

test('filters the model list through the search box', async () => {
  render(ModelSelector, { props: baseProps })

  await fireEvent.input(screen.getByPlaceholderText('Search models...'), {
    target: { value: 'beta' }
  })

  expect(screen.getByText('beta-model.gguf')).toBeTruthy()
  expect(screen.queryByText('alpha-model.gguf')).not.toBeInTheDocument()
})
