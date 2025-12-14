import { experimental_AstroContainer as AstroContainer } from 'astro/container'
import { expect, test } from 'vitest'
// @ts-expect-error - Astro files are not recognized by TypeScript
import Navbar from './Navbar.astro'
import ssr from '@astrojs/svelte/server.js'

test('Navbar renders correctly', async () => {
  const container = await AstroContainer.create()
  container.addServerRenderer({
    name: '@astrojs/svelte',
    renderer: ssr
  })
  container.addClientRenderer({
    name: '@astrojs/svelte',
    entrypoint: '@astrojs/svelte/client-v5.js'
  })
  const result = await container.renderToString(Navbar)

  expect(result).toContain('AI Tools')
  expect(result).toContain('Home')
  expect(result).toContain('Tools')
})

test('Navbar contains navigation links', async () => {
  const container = await AstroContainer.create()
  container.addServerRenderer({
    name: '@astrojs/svelte',
    renderer: ssr
  })
  container.addClientRenderer({
    name: '@astrojs/svelte',
    entrypoint: '@astrojs/svelte/client-v5.js'
  })

  const result = await container.renderToString(Navbar)

  expect(result).toContain('href="/"')
  expect(result).toContain('href="/tools"')
})
