import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { renderMarkdown } from './markdown'

beforeEach(() => {
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('renderMarkdown converts basic markdown to html', () => {
  const html = renderMarkdown('# Heading\n\nsome **bold** text')

  expect(html).toContain('<h1')
  expect(html).toContain('Heading')
  expect(html).toContain('<strong>bold</strong>')
})

test('renderMarkdown honours the gfm/breaks options set at module load', () => {
  // breaks: true means a single newline becomes a <br>
  const html = renderMarkdown('line one\nline two')

  expect(html).toContain('<br>')
})

test('renderMarkdown renders fenced code blocks', () => {
  const html = renderMarkdown('```js\nconst a = 1\n```')

  expect(html).toContain('<pre>')
  expect(html).toContain('<code')
  expect(html).toContain('const a = 1')
})

test('renderMarkdown rewrites \\[ ... \\] block math into katex output', () => {
  const html = renderMarkdown('\\[ x^2 \\]')

  // marked-katex-extension emits katex markup; the raw LaTeX delimiters must be gone.
  expect(html).not.toContain('\\[')
  expect(html).toContain('katex')
})

test('renderMarkdown rewrites \\( ... \\) inline math into katex output', () => {
  const html = renderMarkdown('value is \\( a + b \\) here')

  expect(html).not.toContain('\\(')
  expect(html).toContain('katex')
  expect(html).toContain('here')
})

test('renderMarkdown passes through content with no math untouched by the math preprocessor', () => {
  const html = renderMarkdown('plain $ dollar sign')

  expect(html).toContain('plain $ dollar sign')
})

test('renderMarkdown returns empty content unchanged (empty-input guard)', () => {
  expect(renderMarkdown('')).toBe('')
})

test('renderMarkdown falls back to escaped simple formatting when marked.parse is unavailable', async () => {
  vi.resetModules()
  vi.doMock('marked', () => ({
    marked: {
      // no `parse`, so the fallback branch is taken; `use`/`setOptions` still
      // exist so module-level configuration does not throw.
      use: () => {},
      setOptions: () => {}
    }
  }))

  const { renderMarkdown: fallbackRender } = await import('./markdown')

  const out = fallbackRender('a & b <tag> **bold** *it* `code`\nnext')

  expect(out).toBe(
    'a &amp; b &lt;tag&gt; <strong>bold</strong> <em>it</em> <code>code</code><br>next'
  )

  vi.doUnmock('marked')
  vi.resetModules()
})

test('renderMarkdown returns the raw content when parsing throws', async () => {
  vi.resetModules()
  vi.doMock('marked', () => ({
    marked: {
      use: () => {},
      setOptions: () => {},
      parse: () => {
        throw new Error('boom')
      }
    }
  }))

  const { renderMarkdown: throwingRender } = await import('./markdown')

  expect(throwingRender('# still here')).toBe('# still here')

  vi.doUnmock('marked')
  vi.resetModules()
})

test('module-level marked configuration failure is caught and logged', async () => {
  vi.resetModules()
  const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  vi.doMock('marked', () => ({
    marked: {
      use: () => {
        throw new Error('cannot configure')
      },
      setOptions: () => {},
      parse: (s: string) => `<p>${s}</p>`
    }
  }))

  const { renderMarkdown: stillWorks } = await import('./markdown')

  expect(errorSpy).toHaveBeenCalledWith(
    'Failed to configure marked:',
    expect.any(Error)
  )
  // Rendering still works despite the configuration failure.
  expect(stillWorks('hi')).toBe('<p>hi</p>')

  vi.doUnmock('marked')
  vi.resetModules()
})
