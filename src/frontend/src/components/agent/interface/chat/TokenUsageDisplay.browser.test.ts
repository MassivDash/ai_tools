/**
 * @vitest-environment jsdom
 */

import { render } from '@testing-library/svelte'
import { expect, test } from 'vitest'
import TokenUsageDisplay from './TokenUsageDisplay.svelte'
import type { Component } from 'svelte'

test('does not render when token usage is zero or null', async () => {
  const { queryByText } = render(TokenUsageDisplay as Component, {
    props: {
      tokenUsage: null,
      ctxSize: 4096
    }
  })

  expect(queryByText(/tokens/)).toBeNull()

  const { queryByText: queryByText2 } = render(TokenUsageDisplay as Component, {
    props: {
      tokenUsage: {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0
      },
      ctxSize: 4096
    }
  })

  expect(queryByText2(/tokens/)).toBeNull()
})

test('renders when token usage is greater than zero', async () => {
  const { getByText } = render(TokenUsageDisplay as Component, {
    props: {
      tokenUsage: {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30
      },
      ctxSize: 4096
    }
  })

  // Should show "30 / 4096 tokens (1%)"
  expect(getByText(/30 \/ 4096 tokens/)).toBeTruthy()
})

test('follows token usage and context size updates', async () => {
  const { rerender, getByText, queryByText } = render(
    TokenUsageDisplay as Component,
    {
      props: {
        tokenUsage: {
          prompt_tokens: 10,
          completion_tokens: 10,
          total_tokens: 20
        },
        ctxSize: 200
      }
    }
  )

  expect(getByText('20 / 200 tokens (10%)')).toBeTruthy()

  await rerender({
    tokenUsage: {
      prompt_tokens: 30,
      completion_tokens: 30,
      total_tokens: 60
    },
    ctxSize: 400
  })
  expect(getByText('60 / 400 tokens (15%)')).toBeTruthy()

  // Context size is unknown again — fall back to the bare token count
  await rerender({
    tokenUsage: {
      prompt_tokens: 30,
      completion_tokens: 30,
      total_tokens: 60
    },
    ctxSize: 0
  })
  expect(getByText('60 tokens')).toBeTruthy()

  // Usage cleared — the whole display disappears
  await rerender({ tokenUsage: null, ctxSize: 400 })
  expect(queryByText(/tokens/)).toBeNull()
})

test('renders correctly when ctxSize is 0', async () => {
  const { getByText } = render(TokenUsageDisplay as Component, {
    props: {
      tokenUsage: {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30
      },
      ctxSize: 0
    }
  })

  // Should show "30 tokens"
  expect(getByText('30 tokens')).toBeTruthy()
})
