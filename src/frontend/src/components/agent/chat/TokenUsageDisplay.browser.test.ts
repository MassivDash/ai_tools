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
