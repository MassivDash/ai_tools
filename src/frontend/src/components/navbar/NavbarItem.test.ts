import { experimental_AstroContainer as AstroContainer } from 'astro/container'
import { expect, test } from 'vitest'
// @ts-expect-error - Astro files are not recognized by TypeScript
import NavbarItem from './NavbarItem.astro'

test('Navbar Item internal link', async () => {
  const container = await AstroContainer.create()
  const result = await container.renderToString(NavbarItem, {
    props: {
      id: 'home',
      href: '/',
      external: false
    },
    slots: {
      default: 'Home'
    }
  })

  // Check slot content is rendered
  expect(result).toContain('Home')

  // Check custom element with data attributes
  expect(result).toContain('menu-item')
  expect(result).toContain('data-id="home"')
  expect(result).toContain('data-href="/"')
  expect(result).toContain('data-external="false"')

  // Check list item structure
  expect(result).toContain('navi-link')

  // Check anchor tag attributes
  expect(result).toContain('id="home"')
  expect(result).toContain('href="/"')
  expect(result).toContain('target="_self"')
})

test('Navbar Item external link', async () => {
  const container = await AstroContainer.create()
  const result = await container.renderToString(NavbarItem, {
    props: {
      id: 'external',
      href: 'https://example.com',
      external: true
    },
    slots: {
      default: 'External'
    }
  })

  // Check slot content is rendered
  expect(result).toContain('External')

  // Check custom element with data attributes
  expect(result).toContain('menu-item')
  expect(result).toContain('data-id="external"')
  expect(result).toContain('data-href="https://example.com"')
  expect(result).toContain('data-external="true"')

  // Check list item structure
  expect(result).toContain('navi-link')

  // Check anchor tag attributes
  expect(result).toContain('id="external"')
  expect(result).toContain('href="https://example.com"')
  expect(result).toContain('target="_blank"')
})

test('Navbar Item without external prop (defaults to internal)', async () => {
  const container = await AstroContainer.create()
  const result = await container.renderToString(NavbarItem, {
    props: {
      id: 'about',
      href: '/about'
    },
    slots: {
      default: 'About'
    }
  })

  // Check slot content is rendered
  expect(result).toContain('About')

  // Check anchor tag defaults to _self when external is not provided
  expect(result).toContain('target="_self"')
  expect(result).toContain('href="/about"')
  expect(result).toContain('id="about"')
})
