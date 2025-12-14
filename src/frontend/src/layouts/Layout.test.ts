import { experimental_AstroContainer as AstroContainer } from 'astro/container'
import { expect, test } from 'vitest'
// @ts-expect-error - Astro files are not recognized by TypeScript
import Layout from './Layout.astro'
import ssr from '@astrojs/svelte/server.js'

test('Card with slots', async () => {
  const container = await AstroContainer.create()
  container.addServerRenderer({
    name: '@astrojs/svelte',
    renderer: ssr
  })
  container.addClientRenderer({
    name: '@astrojs/svelte',
    entrypoint: '@astrojs/svelte/client-v5.js'
  })
  const result = await container.renderToString(Layout, {
    props: {
      title: 'Layout Title'
    },
    slots: {
      default: 'Layout content'
    }
  })

  expect(result).toContain('Layout content')
  expect(result).toContain('Layout Title')
})
