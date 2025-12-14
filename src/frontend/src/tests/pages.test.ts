import { experimental_AstroContainer as AstroContainer } from 'astro/container'
import { expect, test } from 'vitest'
// @ts-expect-error - Astro files are not recognized by TypeScript
import IndexPage from '../pages/index.astro'
import ssr from '@astrojs/svelte/server.js'

test('Index Page', async () => {
  const container = await AstroContainer.create()
  container.addServerRenderer({
    name: '@astrojs/svelte',
    renderer: ssr
  })
  container.addClientRenderer({
    name: '@astrojs/svelte',
    entrypoint: '@astrojs/svelte/client-v5.js'
  })

  const result = await container.renderToString(IndexPage)
  expect(result).toContain('AI Tools')
})
