<script lang="ts">
  import { onMount } from 'svelte'
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

  function setTheme(newTheme: Theme) {
    theme.set(newTheme)
  }
</script>

<div class="theme-selector">
  <button
    class="theme-button"
    class:active={currentTheme === 'system'}
    onclick={() => setTheme('system')}
    aria-label="System theme"
    title="System theme"
  >
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
      <line x1="8" y1="21" x2="16" y2="21" />
      <line x1="12" y1="17" x2="12" y2="21" />
    </svg>
  </button>
  <button
    class="theme-button"
    class:active={currentTheme === 'light'}
    onclick={() => setTheme('light')}
    aria-label="Light theme"
    title="Light theme"
  >
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <circle cx="12" cy="12" r="5" />
      <line x1="12" y1="1" x2="12" y2="3" />
      <line x1="12" y1="21" x2="12" y2="23" />
      <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
      <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
      <line x1="1" y1="12" x2="3" y2="12" />
      <line x1="21" y1="12" x2="23" y2="12" />
      <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
      <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
    </svg>
  </button>
  <button
    class="theme-button"
    class:active={currentTheme === 'dark'}
    onclick={() => setTheme('dark')}
    aria-label="Dark theme"
    title="Dark theme"
  >
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </svg>
  </button>
</div>

<style>
  .theme-selector {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem;
    background-color: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 12px;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease;
  }

  .theme-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: none;
    border-radius: 8px;
    background-color: transparent;
    color: var(--text-secondary, #666);
    cursor: pointer;
    transition:
      background-color 0.2s ease,
      color 0.2s ease,
      transform 0.1s ease;
    position: relative;
  }

  .theme-button:hover {
    background-color: var(--bg-tertiary, #f9f9f9);
    color: var(--text-primary, #100f0f);
    transform: scale(1.05);
  }

  .theme-button.active {
    background-color: var(--accent-color, #b12424);
    color: white;
    box-shadow: 0 2px 4px rgba(177, 36, 36, 0.2);
  }

  .theme-button.active:hover {
    background-color: var(--accent-hover, #8f1d1d);
    transform: scale(1.05);
  }

  .theme-button svg {
    width: 18px;
    height: 18px;
  }

  /* Dark theme adjustments */
  :global(.dark) .theme-selector {
    background-color: var(--bg-secondary, #2d2d2d);
    border-color: var(--border-color, #444);
  }

  :global(.dark) .theme-button:hover {
    background-color: var(--bg-tertiary, #252525);
    color: var(--text-primary, #ffffff);
  }

  :global(.dark) .theme-button.active {
    background-color: var(--accent-color, #f44336);
    box-shadow: 0 2px 4px rgba(244, 67, 54, 0.3);
  }

  :global(.dark) .theme-button.active:hover {
    background-color: var(--accent-hover, #da190b);
  }
</style>
