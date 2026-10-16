import { defineConfig } from 'astro/config'

import svelte from '@astrojs/svelte'

// https://astro.build/config
export default defineConfig({
  integrations: [svelte({ extensions: ['.svelte'] })],
  server: { host: true },
  vite: {
    build: {
      rolldownOptions: {
        output: {
          codeSplitting: {
            groups: [
              {
                name: 'vendor-icons',
                test: /@iconify-json/
              },
              {
                name: 'vendor-xlsx',
                test: /xlsx/
              },
              {
                name: 'vendor-chartjs',
                test: /chart\.js/
              },
              {
                name: 'vendor-markdown',
                test: /marked|katex/
              },
              {
                name: 'vendor-heic2any',
                test: /heic2any/
              },
              {
                name: 'vendor',
                test: /node_modules/,
                entriesAware: true
              }
            ]
          }
        }
      }
    }
  }
})