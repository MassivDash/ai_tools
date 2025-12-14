/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import PdfToMarkdown from './pdfToMarkdown.svelte'
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
  render(PdfToMarkdown)

  expect(screen.getByText('PDF to Markdown Converter')).toBeTruthy()
  expect(screen.getByText('Choose PDF file...')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Convert' })).toBeTruthy()
  expect(screen.getByRole('button', { name: /Advanced Options/i })).toBeTruthy()
})

test('convert button is disabled when no file is selected', () => {
  render(PdfToMarkdown)

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  expect(convertButton).toBeDisabled()
})

test('displays selected file name and size', async () => {
  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  await waitFor(() => {
    expect(screen.getByText(/test\.pdf/)).toBeTruthy()
  })
})

test('convert button is enabled when file is selected', async () => {
  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  await waitFor(() => {
    const convertButton = screen.getByRole('button', { name: 'Convert' })
    expect(convertButton).not.toBeDisabled()
  })
})

test('convert button shows loading state', async () => {
  mockedAxios.post.mockImplementation(() => new Promise(() => {})) // Never resolves

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Converting...' })).toBeTruthy()
  })
  expect(fileInput).toBeDisabled()
})

test('successfully converts PDF to markdown', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test Markdown\n\nThis is a test PDF conversion.',
      filename: 'test.pdf',
      token_count: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Markdown Output:')).toBeTruthy()
  })

  const markdownCode = screen.getByText(/# Test Markdown/)
  expect(markdownCode).toBeTruthy()
  expect(markdownCode.textContent).toContain('This is a test PDF conversion.')
  expect(screen.getByText('test.pdf')).toBeTruthy()
})

test('displays token count when count_tokens is enabled', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test Markdown',
      filename: 'test.pdf',
      token_count: 42
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(PdfToMarkdown)

  // Open advanced options
  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })
  fireEvent.click(toggleButton)

  // Enable token counting
  const countTokensCheckbox = screen.getByLabelText(
    /Count tokens \(may slow down conversion for large documents\)/i
  )
  fireEvent.click(countTokensCheckbox)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    // Text is split across elements, so check for both parts
    expect(screen.getByText('Token Count:')).toBeTruthy()
    expect(screen.getByText('42')).toBeTruthy()
  })
})

test('does not display token count when count_tokens is disabled', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test Markdown',
      filename: 'test.pdf',
      token_count: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('test.pdf')).toBeTruthy()
  })

  expect(screen.queryByText(/Token Count:/)).not.toBeInTheDocument()
})

test('displays error message on API failure', async () => {
  const mockError = {
    response: {
      data: {
        error: 'Failed to extract text from PDF'
      }
    }
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Failed to extract text from PDF')).toBeTruthy()
  })
})

test('displays generic error message when error format is unexpected', async () => {
  const mockError = {
    message: 'Network error'
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Network error')).toBeTruthy()
  })
})

test('toggles advanced options', async () => {
  render(PdfToMarkdown)

  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })

  // Initially hidden
  expect(
    screen.queryByText(/Count tokens \(may slow down conversion for large documents\)/i)
  ).not.toBeInTheDocument()

  // Click to show
  fireEvent.click(toggleButton)
  expect(
    screen.getByText(/Count tokens \(may slow down conversion for large documents\)/i)
  ).toBeTruthy()

  // Click to hide
  fireEvent.click(toggleButton)
  await waitFor(() => {
    expect(
      screen.queryByText(/Count tokens \(may slow down conversion for large documents\)/i)
    ).not.toBeInTheDocument()
  })
})

test('sends correct request data with count_tokens disabled', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      filename: 'test.pdf',
      token_count: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })

  const callArgs = mockedAxios.post.mock.calls[0]
  expect(callArgs[0]).toBe('pdf-to-markdown')
  expect(callArgs[1]).toBeInstanceOf(FormData)

  const formData = callArgs[1] as FormData
  expect(formData.get('file')).toBe(file)
  expect(formData.get('count_tokens')).toBe('false')
})

test('sends correct request data with count_tokens enabled', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test',
      filename: 'test.pdf',
      token_count: 10
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(PdfToMarkdown)

  // Open advanced options and enable token counting
  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })
  fireEvent.click(toggleButton)

  const countTokensCheckbox = screen.getByLabelText(
    /Count tokens \(may slow down conversion for large documents\)/i
  )
  fireEvent.click(countTokensCheckbox)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })

  const callArgs = mockedAxios.post.mock.calls[0]
  const formData = callArgs[1] as FormData
  expect(formData.get('count_tokens')).toBe('true')
})

test('downloads markdown file', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test Markdown',
      filename: 'test.pdf',
      token_count: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  // Mock document.createElement and click for download
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

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Markdown Output:')).toBeTruthy()
  })

  const downloadButton = screen.getByRole('button', { name: 'Download markdown file' })
  fireEvent.click(downloadButton)

  // Verify download was triggered
  expect(global.URL.createObjectURL).toHaveBeenCalled()
  expect(mockClick).toHaveBeenCalled()
})

test('shows error when no file is selected and convert is clicked', async () => {
  render(PdfToMarkdown)

  // Try to click convert without selecting a file
  const convertButton = screen.getByRole('button', { name: 'Convert' })
  expect(convertButton).toBeDisabled()
})

test('clears previous results when new file is selected', async () => {
  const mockResponse = {
    data: {
      markdown: '# Test Markdown',
      filename: 'test.pdf',
      token_count: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(PdfToMarkdown)

  const fileInput = screen.getByLabelText(/Choose PDF file/i) as HTMLInputElement
  const file1 = new File(['test content'], 'test1.pdf', { type: 'application/pdf' })

  await fireEvent.change(fileInput, { target: { files: [file1] } })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Markdown Output:')).toBeTruthy()
  })

  // Select a new file
  const file2 = new File(['new content'], 'test2.pdf', { type: 'application/pdf' })
  await fireEvent.change(fileInput, { target: { files: [file2] } })

  // Previous results should be cleared
  expect(screen.queryByText('Markdown Output:')).not.toBeInTheDocument()
  expect(screen.queryByText('test1.pdf')).not.toBeInTheDocument()
})

