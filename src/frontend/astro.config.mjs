import { defineConfig } from 'astro/config'

import svelte from '@astrojs/svelte'

// https://astro.build/config
export default defineConfig({
  integrations: [svelte({ extensions: ['.svelte'] })],
  server: { host: true },
  vite: {
    build: {
      chunkSizeWarningLimit: 3500
    }
  }
})