/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import HtmlToMarkdown from './htmlToMarkdown.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  post: ReturnType<typeof vi.fn>
}

let mockClick: ReturnType<typeof vi.fn>
let mockCreateElement: ReturnType<typeof vi.fn>
const originalCreateElement = document.createElement.bind(document)

const HTML_SAMPLE = '<h1>Hello</h1><p>World</p>'

const successResponse = (overrides: Record<string, unknown> = {}) => ({
  data: {
    markdown: '# Hello\n\nWorld',
    internal_links_count: 0,
    internal_links: [],
    token_count: 0,
    ...overrides
  }
})

const typeHtml = async (value = HTML_SAMPLE) => {
  const textarea = screen.getByPlaceholderText('Paste your HTML here...')
  await fireEvent.input(textarea, { target: { value } })
  return textarea
}

const clickConvert = async () =>
  fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

const openAdvanced = async () =>
  fireEvent.click(screen.getByRole('button', { name: /Advanced Options/i }))

beforeEach(() => {
  vi.clearAllMocks()
  global.URL.createObjectURL = vi.fn(() => 'blob:mock-url')
  global.URL.revokeObjectURL = vi.fn()

  mockClick = vi.fn()
  mockCreateElement = vi.fn((tagName: string) => {
    const el = originalCreateElement(tagName)
    if (tagName === 'a') el.click = mockClick
    return el
  })
  document.createElement =
    mockCreateElement as unknown as Document['createElement']
})

afterEach(() => {
  document.createElement = originalCreateElement as Document['createElement']
})

test('renders initial state with an empty output placeholder', () => {
  render(HtmlToMarkdown)

  expect(screen.getByText('HTML to Markdown Converter')).toBeTruthy()
  expect(screen.getByPlaceholderText('Paste your HTML here...')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Convert' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Clear' })).not.toBeDisabled()
  expect(screen.getByText('Converted markdown will appear here')).toBeTruthy()
  expect(
    screen.queryByRole('button', { name: 'Download markdown file' })
  ).not.toBeInTheDocument()
})

test('convert stays disabled for whitespace-only input and enables for real HTML', async () => {
  render(HtmlToMarkdown)

  await typeHtml('   \n  ')
  expect(screen.getByRole('button', { name: 'Convert' })).toBeDisabled()

  await typeHtml(HTML_SAMPLE)
  expect(screen.getByRole('button', { name: 'Convert' })).not.toBeDisabled()
})

test('toggles advanced options and the disclosure arrow', async () => {
  render(HtmlToMarkdown)

  const toggle = screen.getByRole('button', { name: /Advanced Options/i })
  expect(toggle.textContent).toContain('▶')
  expect(
    screen.queryByLabelText('Extract body content only')
  ).not.toBeInTheDocument()

  await fireEvent.click(toggle)

  expect(toggle.textContent).toContain('▼')
  expect(screen.getByLabelText('Extract body content only')).toBeChecked()
  expect(screen.getByLabelText('Enable preprocessing')).not.toBeChecked()
  expect(screen.getByLabelText(/Count tokens/i)).not.toBeChecked()

  await fireEvent.click(toggle)
  await waitFor(() => {
    expect(
      screen.queryByLabelText('Extract body content only')
    ).not.toBeInTheDocument()
  })
})

test('reveals nested preprocessing controls only when preprocessing is enabled', async () => {
  render(HtmlToMarkdown)
  await openAdvanced()

  expect(
    screen.queryByLabelText('Remove navigation elements')
  ).not.toBeInTheDocument()

  await fireEvent.click(screen.getByLabelText('Enable preprocessing'))

  await waitFor(() => {
    expect(screen.getByLabelText('Remove navigation elements')).toBeTruthy()
  })
  expect(screen.getByLabelText('Remove forms')).toBeTruthy()
  expect(screen.getByLabelText('Preprocessing Preset:')).toHaveValue('minimal')

  await fireEvent.click(screen.getByLabelText('Enable preprocessing'))
  await waitFor(() => {
    expect(
      screen.queryByLabelText('Remove navigation elements')
    ).not.toBeInTheDocument()
  })
})

test('sends the trimmed HTML with default options and a null preset', async () => {
  mockedAxios.post.mockResolvedValue(successResponse())

  render(HtmlToMarkdown)
  await typeHtml(`  ${HTML_SAMPLE}  `)
  await clickConvert()

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('html-to-markdown', {
      html: HTML_SAMPLE,
      extract_body: true,
      enable_preprocessing: false,
      remove_navigation: false,
      remove_forms: false,
      preprocessing_preset: null,
      count_tokens: false
    })
  })
})

test('sends every advanced option including the chosen preset', async () => {
  mockedAxios.post.mockResolvedValue(successResponse())

  render(HtmlToMarkdown)
  await openAdvanced()

  // extract_body defaults to true, so clicking turns it off.
  await fireEvent.click(screen.getByLabelText('Extract body content only'))
  await fireEvent.click(screen.getByLabelText('Enable preprocessing'))
  await fireEvent.click(screen.getByLabelText(/Count tokens/i))
  await fireEvent.click(screen.getByLabelText('Remove navigation elements'))
  await fireEvent.click(screen.getByLabelText('Remove forms'))
  await fireEvent.change(screen.getByLabelText('Preprocessing Preset:'), {
    target: { value: 'aggressive' }
  })

  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('html-to-markdown', {
      html: HTML_SAMPLE,
      extract_body: false,
      enable_preprocessing: true,
      remove_navigation: true,
      remove_forms: true,
      preprocessing_preset: 'aggressive',
      count_tokens: true
    })
  })
})

test('keeps the preset null when preprocessing is off even after choosing one', async () => {
  mockedAxios.post.mockResolvedValue(successResponse())

  render(HtmlToMarkdown)
  await openAdvanced()
  await fireEvent.click(screen.getByLabelText('Enable preprocessing'))
  await fireEvent.change(screen.getByLabelText('Preprocessing Preset:'), {
    target: { value: 'standard' }
  })
  // Turn preprocessing back off; the preset must not be sent.
  await fireEvent.click(screen.getByLabelText('Enable preprocessing'))

  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'html-to-markdown',
      expect.objectContaining({
        enable_preprocessing: false,
        preprocessing_preset: null
      })
    )
  })
})

test('shows loading state and disables the textarea while converting', async () => {
  mockedAxios.post.mockImplementation(() => new Promise(() => {}))

  render(HtmlToMarkdown)
  const textarea = await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Converting...' })).toBeDisabled()
  })
  expect(textarea).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Clear' })).toBeDisabled()
})

test('renders the returned markdown and a download button', async () => {
  mockedAxios.post.mockResolvedValue(successResponse())

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(
      screen.queryByText('Converted markdown will appear here')
    ).not.toBeInTheDocument()
  })

  const output = screen.getByText(/# Hello/)
  expect(output.tagName).toBe('CODE')
  expect(output.textContent).toBe('# Hello\n\nWorld')
  expect(
    screen.getByRole('button', { name: 'Download markdown file' })
  ).toBeTruthy()
})

test('renders a locale-formatted token count when the API returns one', async () => {
  mockedAxios.post.mockResolvedValue(successResponse({ token_count: 12345 }))

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText('Token Count:')).toBeTruthy()
  })
  expect(screen.getByText((12345).toLocaleString())).toBeTruthy()
})

test('omits the token panel when the API omits token_count', async () => {
  const response = successResponse()
  delete (response.data as Record<string, unknown>).token_count
  mockedAxios.post.mockResolvedValue(response)

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText(/# Hello/)).toBeTruthy()
  })
  expect(screen.queryByText('Token Count:')).not.toBeInTheDocument()
})

test('lists internal links with their resolved URLs', async () => {
  mockedAxios.post.mockResolvedValue(
    successResponse({
      internal_links_count: 2,
      internal_links: [
        {
          original: '/about',
          full_url: 'https://example.com/about',
          link_text: 'About'
        },
        {
          original: '/contact',
          full_url: 'https://example.com/contact',
          link_text: 'Contact'
        }
      ]
    })
  )

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText('Internal Links Found: 2')).toBeTruthy()
  })

  const about = screen.getByText('/about')
  expect(about.closest('a')).toHaveAttribute(
    'href',
    'https://example.com/about'
  )
  expect(about.closest('a')).toHaveAttribute('target', '_blank')
  expect(about.closest('a')).toHaveAttribute('rel', 'noopener noreferrer')
  expect(screen.getByText('→ https://example.com/about')).toBeTruthy()
  expect(screen.getByText('→ https://example.com/contact')).toBeTruthy()
})

test('omits the internal links panel when there are none', async () => {
  mockedAxios.post.mockResolvedValue(successResponse())

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText(/# Hello/)).toBeTruthy()
  })
  expect(screen.queryByText(/Internal Links Found/)).not.toBeInTheDocument()
})

test('downloads the markdown as a .md file', async () => {
  mockedAxios.post.mockResolvedValue(successResponse())

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(
      screen.getByRole('button', { name: 'Download markdown file' })
    ).toBeTruthy()
  })

  await fireEvent.click(
    screen.getByRole('button', { name: 'Download markdown file' })
  )

  expect(mockCreateElement).toHaveBeenCalledWith('a')
  const anchor = mockCreateElement.mock.results
    .map((r) => r.value as HTMLElement)
    .find((el) => el.tagName === 'A') as HTMLAnchorElement
  expect(anchor.download).toMatch(/^html_to_markdown_\d+\.md$/)
  expect(anchor.getAttribute('href')).toBe('blob:mock-url')
  expect(mockClick).toHaveBeenCalledTimes(1)
  expect(global.URL.createObjectURL).toHaveBeenCalledWith(expect.any(Blob))
  expect(global.URL.revokeObjectURL).toHaveBeenCalledWith('blob:mock-url')
  expect(document.body.contains(anchor)).toBe(false)
})

test('shows the API error payload and drops any previous result', async () => {
  mockedAxios.post.mockResolvedValueOnce(
    successResponse({
      internal_links_count: 1,
      internal_links: [
        {
          original: '/a',
          full_url: 'https://example.com/a',
          link_text: 'A'
        }
      ],
      token_count: 7
    })
  )

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText('Internal Links Found: 1')).toBeTruthy()
  })

  mockedAxios.post.mockRejectedValueOnce({
    response: { data: { error: 'Malformed HTML document' } },
    message: 'Request failed with status code 400'
  })
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText('Malformed HTML document')).toBeTruthy()
  })
  expect(screen.getByText('Converted markdown will appear here')).toBeTruthy()
  expect(screen.queryByText(/Internal Links Found/)).not.toBeInTheDocument()
  expect(screen.queryByText('Token Count:')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Convert' })).not.toBeDisabled()
})

test('falls back to the error message when there is no response payload', async () => {
  mockedAxios.post.mockRejectedValue({ message: 'Network Error' })

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText('Network Error')).toBeTruthy()
  })
})

test('falls back to a generic message for an opaque failure', async () => {
  mockedAxios.post.mockRejectedValue({})

  render(HtmlToMarkdown)
  await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText('Failed to convert HTML to markdown')).toBeTruthy()
  })
})

test('clear resets input, output, links, tokens and errors', async () => {
  mockedAxios.post.mockResolvedValue(
    successResponse({
      internal_links_count: 1,
      internal_links: [
        {
          original: '/a',
          full_url: 'https://example.com/a',
          link_text: 'A'
        }
      ],
      token_count: 3
    })
  )

  render(HtmlToMarkdown)
  const textarea = await typeHtml()
  await clickConvert()

  await waitFor(() => {
    expect(screen.getByText(/# Hello/)).toBeTruthy()
  })

  await fireEvent.click(screen.getByRole('button', { name: 'Clear' }))

  await waitFor(() => {
    expect(screen.getByText('Converted markdown will appear here')).toBeTruthy()
  })
  expect(textarea).toHaveValue('')
  expect(screen.queryByText(/Internal Links Found/)).not.toBeInTheDocument()
  expect(screen.queryByText('Token Count:')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Convert' })).toBeDisabled()
})
