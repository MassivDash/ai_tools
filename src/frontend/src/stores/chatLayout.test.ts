/**
 * @vitest-environment jsdom
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { get } from 'svelte/store'

type ChatLayoutStore = {
  subscribe: (
    _run: (_value: { height: number; width?: number }) => void
  ) => () => void
  setHeight: (_height: number) => void
  setWidth: (_width: number) => void
  setDimensions: (_height: number, _width: number) => void
}

const DEFAULT_HEIGHT = 600
const DEFAULT_WIDTH = 1000

/**
 * The store is created once at module evaluation time, so every test that cares
 * about initialisation has to re-import the module with a fresh registry.
 */
async function freshStore(): Promise<ChatLayoutStore> {
  vi.resetModules()
  const mod = await import('./chatLayout')
  return mod.chatLayout as unknown as ChatLayoutStore
}

function setViewport(width: number, height: number) {
  Object.defineProperty(window, 'innerWidth', {
    value: width,
    writable: true,
    configurable: true
  })
  Object.defineProperty(window, 'innerHeight', {
    value: height,
    writable: true,
    configurable: true
  })
}

function stored() {
  const raw = localStorage.getItem('chatLayout')
  return raw ? JSON.parse(raw) : null
}

beforeEach(() => {
  localStorage.clear()
  setViewport(1024, 768)
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
  localStorage.clear()
})

test('derives layout from the viewport when nothing is stored', async () => {
  setViewport(800, 900)

  const chatLayout = await freshStore()

  expect(get(chatLayout as never)).toEqual({
    height: Math.floor(900 * 0.6),
    width: 800 - 40
  })
})

test('caps derived width at 1024px on very wide viewports', async () => {
  setViewport(2560, 1000)

  const chatLayout = await freshStore()

  expect(get(chatLayout as never)).toEqual({ height: 600, width: 1024 })
})

test('restores a previously stored layout', async () => {
  localStorage.setItem(
    'chatLayout',
    JSON.stringify({ height: 333, width: 777 })
  )

  const chatLayout = await freshStore()

  expect(get(chatLayout as never)).toEqual({ height: 333, width: 777 })
})

test('falls back to defaults for missing / zero fields in stored layout', async () => {
  localStorage.setItem('chatLayout', JSON.stringify({ height: 0 }))

  const chatLayout = await freshStore()

  expect(get(chatLayout as never)).toEqual({
    height: DEFAULT_HEIGHT,
    width: DEFAULT_WIDTH
  })
})

test('falls back to defaults when the stored value is not valid JSON', async () => {
  localStorage.setItem('chatLayout', '{not json')

  const chatLayout = await freshStore()

  expect(get(chatLayout as never)).toEqual({
    height: DEFAULT_HEIGHT,
    width: DEFAULT_WIDTH
  })
})

test('uses defaults when there is no window (SSR)', async () => {
  localStorage.setItem(
    'chatLayout',
    JSON.stringify({ height: 111, width: 222 })
  )
  vi.stubGlobal('window', undefined)

  const chatLayout = await freshStore()

  // The stored value must be ignored: the SSR guard returns before reading it.
  expect(get(chatLayout as never)).toEqual({
    height: DEFAULT_HEIGHT,
    width: DEFAULT_WIDTH
  })
})

test('setHeight updates the store and persists it', async () => {
  const chatLayout = await freshStore()
  const before = get(chatLayout as never) as { width?: number }

  chatLayout.setHeight(742)

  expect(get(chatLayout as never)).toEqual({
    height: 742,
    width: before.width
  })
  expect(stored()).toEqual({ height: 742, width: before.width })
})

test('setWidth updates the store and persists it, keeping the height', async () => {
  const chatLayout = await freshStore()
  const before = get(chatLayout as never) as { height: number }

  chatLayout.setWidth(480)

  expect(get(chatLayout as never)).toEqual({
    height: before.height,
    width: 480
  })
  expect(stored()).toEqual({ height: before.height, width: 480 })
})

test('setDimensions updates both values and persists them', async () => {
  const chatLayout = await freshStore()

  chatLayout.setDimensions(250, 350)

  expect(get(chatLayout as never)).toEqual({ height: 250, width: 350 })
  expect(stored()).toEqual({ height: 250, width: 350 })
})

test('successive setters accumulate onto the persisted layout', async () => {
  const chatLayout = await freshStore()

  chatLayout.setHeight(100)
  chatLayout.setWidth(200)

  expect(get(chatLayout as never)).toEqual({ height: 100, width: 200 })
  expect(stored()).toEqual({ height: 100, width: 200 })
})

test('a fresh store picks up whatever the setters persisted', async () => {
  const first = await freshStore()
  first.setDimensions(432, 543)

  const second = await freshStore()

  expect(get(second as never)).toEqual({ height: 432, width: 543 })
})

test('setters still update state when localStorage is unavailable', async () => {
  const chatLayout = await freshStore()
  vi.stubGlobal('localStorage', undefined)

  chatLayout.setHeight(11)
  chatLayout.setWidth(22)
  chatLayout.setDimensions(33, 44)

  expect(get(chatLayout as never)).toEqual({ height: 33, width: 44 })
})

test('notifies subscribers on every change', async () => {
  const chatLayout = await freshStore()
  const seen: Array<{ height: number; width?: number }> = []

  const unsubscribe = chatLayout.subscribe((value) => seen.push({ ...value }))
  chatLayout.setHeight(120)
  chatLayout.setDimensions(130, 140)
  unsubscribe()
  chatLayout.setHeight(999)

  expect(seen).toHaveLength(3)
  expect(seen[1].height).toBe(120)
  expect(seen[2]).toEqual({ height: 130, width: 140 })
})
