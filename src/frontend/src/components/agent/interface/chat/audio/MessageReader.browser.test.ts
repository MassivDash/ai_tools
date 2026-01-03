/**
 * @vitest-environment jsdom
 */

import { render, fireEvent } from '@testing-library/svelte'
import { expect, test, vi, describe } from 'vitest'
import MessageReader from './MessageReader.svelte'

describe('MessageReader Component', () => {
  test('renders correctly when enabled', () => {
    const { getByTitle } = render(MessageReader, {
      props: {
        enabled: true,
        speaking: false,
        onToggle: vi.fn(),
        onStop: vi.fn()
      }
    })

    expect(getByTitle('Read Messages: On')).toBeTruthy()
  })

  test('renders correctly when disabled', () => {
    const { getByTitle } = render(MessageReader, {
      props: {
        enabled: false,
        speaking: false,
        onToggle: vi.fn(),
        onStop: vi.fn()
      }
    })

    expect(getByTitle('Read Messages: Off')).toBeTruthy()
  })

  test('renders stop button when speaking', () => {
    const { getByTitle } = render(MessageReader, {
      props: {
        enabled: true,
        speaking: true,
        onToggle: vi.fn(),
        onStop: vi.fn()
      }
    })

    expect(getByTitle('Stop Speaking')).toBeTruthy()
  })

  test('calls onToggle when clicked', async () => {
    const onToggle = vi.fn()
    const { getByTitle } = render(MessageReader, {
      props: {
        enabled: false,
        speaking: false,
        onToggle,
        onStop: vi.fn()
      }
    })

    const button = getByTitle('Read Messages: Off')
    await fireEvent.click(button)

    expect(onToggle).toHaveBeenCalled()
  })

  test('calls onStop when speaking and clicked', async () => {
    const onStop = vi.fn()
    const onToggle = vi.fn()
    const { getByTitle } = render(MessageReader, {
      props: {
        enabled: true,
        speaking: true,
        onToggle,
        onStop
      }
    })

    const button = getByTitle('Stop Speaking')
    await fireEvent.click(button)

    expect(onStop).toHaveBeenCalled()
    expect(onToggle).not.toHaveBeenCalled()
  })
})
