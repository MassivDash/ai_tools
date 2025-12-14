<script lang="ts">
  import { onMount } from 'svelte'
  import Select from '../ui/Select.svelte'
  import { theme, type Theme } from '../../stores/theme'

  let currentTheme: Theme = 'system'

  onMount(() => {
    // Get the actual stored theme preference (not the effective theme)
    const stored = localStorage.getItem('theme')
    if (stored && ['system', 'light', 'dark'].includes(stored)) {
      currentTheme = stored as Theme
    } else {
      currentTheme = 'system'
    }

    // Subscribe to theme changes
    const unsubscribe = theme.subscribe((value) => {
      currentTheme = value
    })
    return unsubscribe
  })

  function handleThemeChange(event: CustomEvent<{ value: string }>) {
    const newTheme = event.detail.value as Theme
    theme.set(newTheme)
  }

  const themeOptions = [
    { value: 'system', label: '🌓 System' },
    { value: 'light', label: '☀️ Light' },
    { value: 'dark', label: '🌙 Dark' }
  ]
</script>

<div class="theme-selector-wrapper">
  <Select
    id="theme-selector"
    value={currentTheme}
    options={themeOptions}
    on:change={handleThemeChange}
  />
</div>

<style>
  .theme-selector-wrapper :global(#theme-selector) {
    min-width: 140px;
  }
</style>
