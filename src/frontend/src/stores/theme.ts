import { writable } from 'svelte/store'

export type Theme = 'system' | 'light' | 'dark'

function getSystemTheme(): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'light'
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light'
}

function getStoredTheme(): Theme | null {
  if (typeof window === 'undefined') return null
  const stored = localStorage.getItem('theme') as Theme | null
  return stored && ['system', 'light', 'dark'].includes(stored) ? stored : null
}

function getEffectiveTheme(theme: Theme): 'light' | 'dark' {
  return theme === 'system' ? getSystemTheme() : theme
}

function applyTheme(theme: 'light' | 'dark') {
  if (typeof document === 'undefined') return
  if (theme === 'light') {
    document.documentElement.classList.remove('dark')
  } else {
    document.documentElement.classList.add('dark')
  }
}

function initTheme(): Theme {
  const stored = getStoredTheme()
  const initialTheme = stored || 'system'
  applyTheme(getEffectiveTheme(initialTheme))
  return initialTheme
}

function createThemeStore() {
  const { subscribe, set, update } = writable<Theme>(initTheme())

  return {
    subscribe,
    set: (theme: Theme) => {
      set(theme)
      localStorage.setItem('theme', theme)
      applyTheme(getEffectiveTheme(theme))
    },
    update
  }
}

export const theme = createThemeStore()

// Listen for system theme changes
if (typeof window !== 'undefined') {
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  mediaQuery.addEventListener('change', () => {
    theme.subscribe((currentTheme) => {
      if (currentTheme === 'system') {
        applyTheme(getEffectiveTheme(currentTheme))
      }
    })()
  })
}
