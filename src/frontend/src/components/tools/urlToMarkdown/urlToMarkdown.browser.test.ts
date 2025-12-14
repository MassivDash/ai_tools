/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import UrlToMarkdown from './urlToMarkdown.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as {
  post: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  // Mock URL.createObjectURL and revokeObjectURL
  global.URL.createObjectURL = vi.fn(() => 'blob:mock-url')
  global.URL.revokeObjectURL = vi.fn()
})

test('renders component with initial state', () => {
  render(UrlToMarkdown)

  expect(screen.getByText('URL to Markdown Converter')).toBeTruthy()
  expect(
    screen.getByPlaceholderText('Enter a URL to convert to markdown...')
  ).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Convert' })).toBeTruthy()
  expect(screen.getByRole('button', { name: /Advanced Options/i })).toBeTruthy()
})

test('convert button is disabled when URL is empty', () => {
  render(UrlToMarkdown)

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  expect(convertButton).toBeDisabled()
})

test('convert button is enabled when URL is provided', async () => {
  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  expect(convertButton).not.toBeDisabled()
})

test('convert button shows loading state', async () => {
  mockedAxios.post.mockImplementation(() => new Promise(() => {})) // Never resolves

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Converting...' })).toBeTruthy()
  })
  expect(urlInput).toBeDisabled()
})

test('successfully converts URL to markdown', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test Markdown\n\nThis is a test.',
      url: 'https://example.com',
      internal_links_count: 2,
      internal_links: [
        { original: '/about', full_url: 'https://example.com/about' },
        { original: '/contact', full_url: 'https://example.com/contact' }
      ]
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Markdown Output:')).toBeTruthy()
  })

  const markdownCode = screen.getByText(/# Test Markdown/)
  expect(markdownCode).toBeTruthy()
  expect(markdownCode.textContent).toContain('This is a test.')
  expect(screen.getByText('https://example.com')).toBeTruthy()
  expect(screen.getByText('Internal Links Found: 2')).toBeTruthy()
  expect(screen.getByText('/about')).toBeTruthy()
  expect(screen.getByText('/contact')).toBeTruthy()
})

test('displays error message on API failure', async () => {
  const mockError = {
    response: {
      data: {
        error: 'Invalid URL provided'
      }
    }
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Invalid URL provided')).toBeTruthy()
  })
})

test('displays generic error message when error format is unexpected', async () => {
  const mockError = {
    message: 'Network error'
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Network error')).toBeTruthy()
  })
})

test('triggers conversion on Enter key press', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      url: 'https://example.com',
      internal_links_count: 0,
      internal_links: []
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })
  fireEvent.keyPress(urlInput, { key: 'Enter' })

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })
})

test('does not trigger conversion on Shift+Enter', async () => {
  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })
  fireEvent.keyPress(urlInput, { key: 'Enter', shiftKey: true })

  // Wait a bit to ensure no API call is made
  await new Promise((resolve) => setTimeout(resolve, 100))

  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('toggles advanced options', async () => {
  render(UrlToMarkdown)

  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })

  // Initially hidden
  expect(
    screen.queryByText('Extract body content only')
  ).not.toBeInTheDocument()

  // Click to show
  fireEvent.click(toggleButton)
  expect(screen.getByText('Extract body content only')).toBeTruthy()

  // Click to hide
  fireEvent.click(toggleButton)
  await waitFor(() => {
    expect(
      screen.queryByText('Extract body content only')
    ).not.toBeInTheDocument()
  })
})

test('shows preprocessing options when preprocessing is enabled', async () => {
  render(UrlToMarkdown)

  // Open advanced options
  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })
  fireEvent.click(toggleButton)

  // Enable preprocessing checkbox
  const preprocessingCheckbox = screen.getByLabelText('Enable preprocessing')
  fireEvent.click(preprocessingCheckbox)

  await waitFor(() => {
    expect(screen.getByText('Remove navigation elements')).toBeTruthy()
    expect(screen.getByText('Remove forms')).toBeTruthy()
    expect(screen.getByLabelText('Preprocessing Preset:')).toBeTruthy()
  })
})

test('sends correct request data with default options', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      url: 'https://example.com',
      internal_links_count: 0,
      internal_links: []
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'url-to-markdown',
      expect.objectContaining({
        url: 'https://example.com',
        extract_body: false,
        enable_preprocessing: false,
        remove_navigation: false,
        remove_forms: false,
        preprocessing_preset: null,
        follow_links: false
      }),
      expect.objectContaining({
        responseType: 'json'
      })
    )
  })
})

test('sends correct request data with advanced options', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      url: 'https://example.com',
      internal_links_count: 0,
      internal_links: []
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  // Open advanced options
  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })
  fireEvent.click(toggleButton)

  // Configure options
  fireEvent.click(screen.getByLabelText('Extract body content only'))
  fireEvent.click(screen.getByLabelText('Enable preprocessing'))
  fireEvent.click(screen.getByLabelText('Remove navigation elements'))
  fireEvent.click(screen.getByLabelText('Remove forms'))

  const presetSelect = screen.getByLabelText('Preprocessing Preset:')
  fireEvent.change(presetSelect, { target: { value: 'aggressive' } })

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'url-to-markdown',
      expect.objectContaining({
        url: 'https://example.com',
        extract_body: false,
        enable_preprocessing: true,
        remove_navigation: true,
        remove_forms: true,
        preprocessing_preset: 'aggressive',
        follow_links: false
      }),
      expect.objectContaining({
        responseType: 'json'
      })
    )
  })
})

test('handles follow links option and downloads zip file', async () => {
  const mockBlob = new Blob(['zip content'], { type: 'application/zip' })
  const mockResponse = {
    data: mockBlob
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  // Mock document.createElement and click
  const mockClick = vi.fn()
  const originalCreateElement = document.createElement.bind(document)
  const mockCreateElement = vi.fn((tagName: string) => {
    if (tagName === 'a') {
      const anchor = originalCreateElement('a')
      anchor.click = mockClick
      return anchor
    }
    return originalCreateElement(tagName)
  })

  document.createElement = mockCreateElement

  render(UrlToMarkdown)

  // Open advanced options
  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })
  fireEvent.click(toggleButton)

  // Enable follow links
  fireEvent.click(
    screen.getByLabelText('Follow internal links (creates zip file)')
  )

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'url-to-markdown',
      expect.any(Object),
      expect.objectContaining({
        responseType: 'blob'
      })
    )
  })

  await waitFor(() => {
    expect(mockCreateElement).toHaveBeenCalledWith('a')
    expect(mockClick).toHaveBeenCalled()
    expect(screen.getByText('Zip file downloaded successfully!')).toBeTruthy()
  })

  // Restore original
  document.createElement = originalCreateElement
})

test('downloads markdown file when download button is clicked', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test Markdown',
      url: 'https://example.com',
      internal_links_count: 0,
      internal_links: []
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  // Mock document.createElement and click
  const mockClick = vi.fn()
  const originalCreateElement = document.createElement.bind(document)
  const mockCreateElement = vi.fn((tagName: string) => {
    if (tagName === 'a') {
      const anchor = originalCreateElement('a')
      anchor.click = mockClick
      return anchor
    }
    return originalCreateElement(tagName)
  })

  document.createElement = mockCreateElement

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Markdown Output:')).toBeTruthy()
  })

  const downloadButton = screen.getByRole('button', {
    name: 'Download markdown file'
  })
  fireEvent.click(downloadButton)

  await waitFor(() => {
    expect(mockCreateElement).toHaveBeenCalledWith('a')
    expect(mockClick).toHaveBeenCalled()
  })

  // Restore original
  document.createElement = originalCreateElement
})

test('does not download when markdown is empty', () => {
  render(UrlToMarkdown)

  // Download button should not be visible when there's no markdown
  expect(
    screen.queryByRole('button', { name: 'Download markdown file' })
  ).not.toBeInTheDocument()
})

test('trims URL before sending request', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      url: 'https://example.com',
      internal_links_count: 0,
      internal_links: []
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, {
    target: { value: '  https://example.com  ' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'url-to-markdown',
      expect.objectContaining({
        url: 'https://example.com'
      }),
      expect.any(Object)
    )
  })
})

test('displays converted URL link', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      url: 'https://converted-url.com',
      internal_links_count: 0,
      internal_links: []
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    const link = screen.getByText('https://converted-url.com')
    expect(link).toBeTruthy()
    expect(link.closest('a')).toHaveAttribute(
      'href',
      'https://converted-url.com'
    )
    expect(link.closest('a')).toHaveAttribute('target', '_blank')
    expect(link.closest('a')).toHaveAttribute('rel', 'noopener noreferrer')
  })
})

test('displays internal links correctly', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      url: 'https://example.com',
      internal_links_count: 2,
      internal_links: [
        { original: '/about', full_url: 'https://example.com/about' },
        { original: '/contact', full_url: 'https://example.com/contact' }
      ]
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Internal Links Found: 2')).toBeTruthy()

    const aboutLink = screen.getByText('/about')
    expect(aboutLink.closest('a')).toHaveAttribute(
      'href',
      'https://example.com/about'
    )

    const contactLink = screen.getByText('/contact')
    expect(contactLink.closest('a')).toHaveAttribute(
      'href',
      'https://example.com/contact'
    )
  })
})

test('does not display internal links section when count is 0', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      url: 'https://example.com',
      internal_links_count: 0,
      internal_links: []
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(UrlToMarkdown)

  const urlInput = screen.getByPlaceholderText(
    'Enter a URL to convert to markdown...'
  )
  await fireEvent.input(urlInput, { target: { value: 'https://example.com' } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Markdown Output:')).toBeTruthy()
  })

  expect(screen.queryByText(/Internal Links Found/i)).not.toBeInTheDocument()
})
