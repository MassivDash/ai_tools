/**
 * @vitest-environment jsdom
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { get } from 'svelte/store'
import type { Theme } from './theme'

type ThemeStore = {
  subscribe: (_run: (_value: Theme) => void) => () => void
  set: (_theme: Theme) => void
  update: (_fn: (_current: Theme) => Theme) => void
}

type MediaQueryMock = {
  matches: boolean
  listeners: Array<() => void>
  addEventListener: ReturnType<typeof vi.fn>
  removeEventListener: ReturnType<typeof vi.fn>
}

let mediaQuery: MediaQueryMock
let matchMedia: ReturnType<typeof vi.fn>

function mockMatchMedia(matches: boolean) {
  mediaQuery = {
    matches,
    listeners: [],
    addEventListener: vi.fn((_type: string, cb: () => void) => {
      mediaQuery.listeners.push(cb)
    }),
    removeEventListener: vi.fn()
  }
  matchMedia = vi.fn(() => mediaQuery)
  vi.stubGlobal('matchMedia', matchMedia)
}

/**
 * `theme.ts` reads localStorage / matchMedia and mutates <html> at module
 * evaluation time, so each scenario needs a freshly evaluated module.
 */
async function freshTheme(): Promise<ThemeStore> {
  vi.resetModules()
  const mod = await import('./theme')
  return mod.theme as unknown as ThemeStore
}

function isDark() {
  return document.documentElement.classList.contains('dark')
}

beforeEach(() => {
  localStorage.clear()
  document.documentElement.classList.remove('dark')
  mockMatchMedia(false)
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
  localStorage.clear()
  document.documentElement.classList.remove('dark')
})

test('defaults to "system" and applies the light system preference', async () => {
  mockMatchMedia(false)

  const theme = await freshTheme()

  expect(get(theme as never)).toBe('system')
  expect(matchMedia).toHaveBeenCalledWith('(prefers-color-scheme: dark)')
  expect(isDark()).toBe(false)
})

test('defaults to "system" and applies the dark system preference', async () => {
  mockMatchMedia(true)

  const theme = await freshTheme()

  expect(get(theme as never)).toBe('system')
  expect(isDark()).toBe(true)
})

test('restores a stored "dark" preference and ignores the system preference', async () => {
  localStorage.setItem('theme', 'dark')
  mockMatchMedia(false)

  const theme = await freshTheme()

  expect(get(theme as never)).toBe('dark')
  expect(isDark()).toBe(true)
})

test('restores a stored "light" preference even when the system is dark', async () => {
  localStorage.setItem('theme', 'light')
  mockMatchMedia(true)
  document.documentElement.classList.add('dark')

  const theme = await freshTheme()

  expect(get(theme as never)).toBe('light')
  expect(isDark()).toBe(false)
})

test('restores a stored "system" preference', async () => {
  localStorage.setItem('theme', 'system')
  mockMatchMedia(true)

  const theme = await freshTheme()

  expect(get(theme as never)).toBe('system')
  expect(isDark()).toBe(true)
})

test('ignores an unrecognised stored value and falls back to "system"', async () => {
  localStorage.setItem('theme', 'chartreuse')
  mockMatchMedia(true)

  const theme = await freshTheme()

  expect(get(theme as never)).toBe('system')
  expect(isDark()).toBe(true)
})

test('set("dark") stores the choice and adds the dark class', async () => {
  mockMatchMedia(false)
  const theme = await freshTheme()

  theme.set('dark')

  expect(get(theme as never)).toBe('dark')
  expect(localStorage.getItem('theme')).toBe('dark')
  expect(isDark()).toBe(true)
})

test('set("light") stores the choice and removes the dark class', async () => {
  mockMatchMedia(true)
  const theme = await freshTheme()
  expect(isDark()).toBe(true)

  theme.set('light')

  expect(get(theme as never)).toBe('light')
  expect(localStorage.getItem('theme')).toBe('light')
  expect(isDark()).toBe(false)
})

test('set("system") re-resolves against the system preference', async () => {
  mockMatchMedia(true)
  const theme = await freshTheme()

  theme.set('light')
  expect(isDark()).toBe(false)

  theme.set('system')

  expect(get(theme as never)).toBe('system')
  expect(localStorage.getItem('theme')).toBe('system')
  expect(isDark()).toBe(true)
})

test('subscribers are notified of each change', async () => {
  mockMatchMedia(false)
  const theme = await freshTheme()
  const seen: Theme[] = []

  const unsubscribe = theme.subscribe((value) => seen.push(value))
  theme.set('dark')
  theme.set('light')
  unsubscribe()
  theme.set('dark')

  expect(seen).toEqual(['system', 'dark', 'light'])
})

test('update mutates the store value without persisting', async () => {
  mockMatchMedia(false)
  const theme = await freshTheme()

  theme.update(() => 'dark')

  expect(get(theme as never)).toBe('dark')
  // `update` is passed straight through from the writable: no side effects.
  expect(localStorage.getItem('theme')).toBe(null)
  expect(isDark()).toBe(false)
})

test('registers a listener for system theme changes', async () => {
  mockMatchMedia(false)
  await freshTheme()

  expect(mediaQuery.addEventListener).toHaveBeenCalledWith(
    'change',
    expect.any(Function)
  )
  expect(mediaQuery.listeners).toHaveLength(1)
})

test('a system change re-applies the theme while on "system"', async () => {
  mockMatchMedia(false)
  await freshTheme()
  expect(isDark()).toBe(false)

  mediaQuery.matches = true
  mediaQuery.listeners[0]()

  expect(isDark()).toBe(true)

  mediaQuery.matches = false
  mediaQuery.listeners[0]()

  expect(isDark()).toBe(false)
})

test('a system change is ignored when an explicit theme is selected', async () => {
  mockMatchMedia(false)
  const theme = await freshTheme()

  theme.set('light')
  mediaQuery.matches = true
  mediaQuery.listeners[0]()

  expect(isDark()).toBe(false)
  expect(get(theme as never)).toBe('light')
})

test('does not touch the DOM or register listeners without a window (SSR)', async () => {
  localStorage.setItem('theme', 'dark')
  mockMatchMedia(true)
  const listenerCountBefore = mediaQuery.listeners.length
  vi.stubGlobal('window', undefined)
  vi.stubGlobal('document', undefined)

  const theme = await freshTheme()

  // No window means no stored theme is read, so it falls back to 'system',
  // and no document means applyTheme is a no-op.
  expect(get(theme as never)).toBe('system')
  expect(mediaQuery.listeners).toHaveLength(listenerCountBefore)
  expect(matchMedia).not.toHaveBeenCalled()
})
