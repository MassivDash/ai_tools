/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'

// `src/stores/theme.ts` reads matchMedia at module-evaluation time, so the stub has
// to exist before the module graph is imported — hence vi.hoisted.
const mm = vi.hoisted(() => {
  const listeners: Array<() => void> = []
  const state = { matches: false }
  return { listeners, state }
})

vi.stubGlobal('matchMedia', (query: string) => ({
  matches: mm.state.matches,
  media: query,
  onchange: null,
  addEventListener: (_type: string, cb: () => void) => {
    mm.listeners.push(cb)
  },
  removeEventListener: () => {},
  addListener: () => {},
  removeListener: () => {},
  dispatchEvent: () => false
}))

const ThemeSelector = (await import('./ThemeSelector.svelte')).default
const { theme } = await import('../../stores/theme')

const buttons = (container: HTMLElement) =>
  Array.from(container.querySelectorAll('button.theme-button'))

const byLabel = (container: HTMLElement, label: string) =>
  container.querySelector(`button[aria-label="${label}"]`) as HTMLButtonElement

beforeEach(() => {
  localStorage.clear()
  mm.state.matches = false
  document.documentElement.classList.remove('dark')
  theme.set('system')
  localStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('renders one button per theme option with accessible labels', () => {
  const { container } = render(ThemeSelector)

  expect(buttons(container)).toHaveLength(3)
  expect(byLabel(container, 'System theme')).toBeTruthy()
  expect(byLabel(container, 'Light theme')).toBeTruthy()
  expect(byLabel(container, 'Dark theme')).toBeTruthy()
})

test('marks the system button active when nothing is stored', async () => {
  const { container } = render(ThemeSelector)

  await waitFor(() => {
    expect(byLabel(container, 'System theme')).toHaveClass('active')
  })
  expect(byLabel(container, 'Light theme')).not.toHaveClass('active')
  expect(byLabel(container, 'Dark theme')).not.toHaveClass('active')
})

test('marks the stored theme active on mount', async () => {
  // The store is a module singleton that seeds itself from localStorage, so a
  // realistic mount has both in agreement.
  localStorage.setItem('theme', 'dark')
  theme.set('dark')

  const { container } = render(ThemeSelector)

  await waitFor(() => {
    expect(byLabel(container, 'Dark theme')).toHaveClass('active')
  })
  expect(byLabel(container, 'System theme')).not.toHaveClass('active')
})

test('the store value wins over a disagreeing localStorage entry on mount', async () => {
  // onMount reads localStorage first, then subscribes to the store — and the
  // subscription fires synchronously, so the store value is what ends up rendered.
  localStorage.setItem('theme', 'dark')
  theme.set('light')
  localStorage.setItem('theme', 'dark')

  const { container } = render(ThemeSelector)

  await waitFor(() => {
    expect(byLabel(container, 'Light theme')).toHaveClass('active')
  })
  expect(byLabel(container, 'Dark theme')).not.toHaveClass('active')
})

test('an unrecognised stored value does not activate a bogus option', async () => {
  localStorage.setItem('theme', 'chartreuse')

  const { container } = render(ThemeSelector)

  await waitFor(() => {
    expect(byLabel(container, 'System theme')).toHaveClass('active')
  })
  expect(
    buttons(container).filter((b) => b.classList.contains('active'))
  ).toHaveLength(1)
})

test('clicking the light button stores the choice and removes the dark class', async () => {
  document.documentElement.classList.add('dark')
  const { container } = render(ThemeSelector)

  await fireEvent.click(byLabel(container, 'Light theme'))

  expect(localStorage.getItem('theme')).toBe('light')
  expect(document.documentElement.classList.contains('dark')).toBe(false)
  await waitFor(() => {
    expect(byLabel(container, 'Light theme')).toHaveClass('active')
  })
  expect(byLabel(container, 'System theme')).not.toHaveClass('active')
})

test('clicking the dark button stores the choice and adds the dark class', async () => {
  const { container } = render(ThemeSelector)

  await fireEvent.click(byLabel(container, 'Dark theme'))

  expect(localStorage.getItem('theme')).toBe('dark')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  await waitFor(() => {
    expect(byLabel(container, 'Dark theme')).toHaveClass('active')
  })
})

test('clicking the system button resolves the class from the media query', async () => {
  mm.state.matches = true // system prefers dark
  const { container } = render(ThemeSelector)

  await fireEvent.click(byLabel(container, 'Light theme'))
  expect(document.documentElement.classList.contains('dark')).toBe(false)

  await fireEvent.click(byLabel(container, 'System theme'))

  expect(localStorage.getItem('theme')).toBe('system')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  await waitFor(() => {
    expect(byLabel(container, 'System theme')).toHaveClass('active')
  })
})

test('the active button follows external store updates', async () => {
  const { container } = render(ThemeSelector)

  theme.set('light')

  await waitFor(() => {
    expect(byLabel(container, 'Light theme')).toHaveClass('active')
  })

  theme.set('dark')

  await waitFor(() => {
    expect(byLabel(container, 'Dark theme')).toHaveClass('active')
  })
})

test('exactly one option is ever active', async () => {
  const { container } = render(ThemeSelector)

  for (const label of ['Light theme', 'Dark theme', 'System theme']) {
    await fireEvent.click(byLabel(container, label))
    await waitFor(() => {
      expect(
        buttons(container).filter((b) => b.classList.contains('active'))
      ).toHaveLength(1)
    })
  }
})
