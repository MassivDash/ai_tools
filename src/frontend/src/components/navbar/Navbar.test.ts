import { experimental_AstroContainer as AstroContainer } from 'astro/container'
import { expect, test } from 'vitest'
import Navbar from './Navbar.astro'

test('Navbar renders correctly', async () => {
  const container = await AstroContainer.create()
  const result = await container.renderToString(Navbar)

  expect(result).toContain('AI Tools')
  expect(result).toContain('Home')
  expect(result).toContain('Tools')
  expect(result).toContain('RAG')
})

test('Navbar contains navigation links', async () => {
  const container = await AstroContainer.create()
  const result = await container.renderToString(Navbar)

  expect(result).toContain('href="/"')
  expect(result).toContain('href="/tools"')
  expect(result).toContain('href="/rag"')
})
