// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { describe, it, expect, vi } from 'vitest'
import Round3Buzzer from './Round3Buzzer.svelte'

describe('Round3Buzzer', () => {
  it('renders the buzzer prompt', () => {
    render(Round3Buzzer, { props: { onBuzzIn: vi.fn() } })

    expect(screen.getByRole('button')).toHaveTextContent('BUZZ!')
    expect(screen.getByText('Be the first to buzz in!')).toBeInTheDocument()
  })

  it('calls onBuzzIn once per click', async () => {
    const onBuzzIn = vi.fn()
    render(Round3Buzzer, { props: { onBuzzIn } })

    await fireEvent.click(screen.getByRole('button'))
    expect(onBuzzIn).toHaveBeenCalledTimes(1)

    await fireEvent.click(screen.getByRole('button'))
    expect(onBuzzIn).toHaveBeenCalledTimes(2)
  })
})
