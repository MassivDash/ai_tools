// @vitest-environment jsdom
import { render, screen, act } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import Timer from './Timer.svelte'

describe('Timer Component', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.runOnlyPendingTimers()
    vi.useRealTimers()
  })

  it('renders initial time correctly', () => {
    const startTime = Date.now() / 1000
    render(Timer, { startTime, duration: 60 })

    expect(screen.getByText('60')).toBeInTheDocument()
    expect(screen.getByText('seconds')).toBeInTheDocument()
  })

  it('counts down after 1 second', async () => {
    const startTime = Date.now() / 1000
    render(Timer, { startTime, duration: 60 })

    await act(() => {
      vi.advanceTimersByTime(1000)
    })

    expect(screen.getByText('59')).toBeInTheDocument()
  })

  // Regression test for the "immediate update" fix
  it('updates immediately when startTime changes', async () => {
    const startTime1 = Date.now() / 1000
    const { component } = render(Timer, { startTime: startTime1, duration: 60 })

    expect(screen.getByText('60')).toBeInTheDocument()

    // Simulate 30 seconds passing for the first timer
    await act(() => {
      vi.advanceTimersByTime(30000)
    })
    expect(screen.getByText('30')).toBeInTheDocument()

    // Now switch to a NEW start time (simulate next player)
    // We need to update the prop. In Svelte 5 with testing-library, we re-render or update props manually if possible,
    // but the simplest way with render result is using component.$set if it was Svelte 4,
    // or just rerendering with new props if we were using a parent.
    // For this test, let's remount or assume prop reactivity.
    // Ideally we test the component's reactivity.
    // Let's destroy and re-render to simulate parent passing new props or use `@testing-library/svelte`'s rerender if available.
    // Actually, `render` returns `rerender`.

    const { rerender } = render(Timer, { startTime: startTime1, duration: 60 })

    // reset timers to now for the new start time
    const newStartTime = (Date.now() + 100000) / 1000 // future? no just a different time.
    // Let's simply say "now" is the new start time.
    const now = Date.now() / 1000

    await act(() => {
      rerender({ startTime: now, duration: 60 })
    })

    // Should be back to 60 IMMEDIATELY, not waiting for interval
    expect(screen.getByText('60')).toBeInTheDocument()
  })

  it('adds critical class when time is 10 or less', async () => {
    const startTime = Date.now() / 1000
    render(Timer, { startTime, duration: 15 }) // Start with 15s

    // Advance 6 seconds -> 9s left
    await act(() => {
      vi.advanceTimersByTime(6000)
    })

    const timerDiv = screen.getByText('seconds').parentElement
    expect(timerDiv).toHaveClass('critical')
  })

  it('calls onTimeout when time reaches 0', async () => {
    const onTimeout = vi.fn()
    const startTime = Date.now() / 1000
    render(Timer, { startTime, duration: 5, onTimeout })

    await act(() => {
      vi.advanceTimersByTime(5000)
    })

    expect(screen.getByText('0')).toBeInTheDocument()
    expect(onTimeout).toHaveBeenCalled()
  })
})
