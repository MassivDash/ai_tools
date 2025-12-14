/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import JsonToToon from './jsonToToon.svelte'
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
  render(JsonToToon)

  expect(screen.getByText('JSON to TOON Converter')).toBeTruthy()
  expect(screen.getByText('📝 Paste JSON')).toBeTruthy()
  expect(screen.getByText('📁 Upload File')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Convert' })).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Clear' })).toBeTruthy()
})

test('defaults to paste mode', () => {
  render(JsonToToon)

  expect(screen.getByPlaceholderText('Paste your JSON here...')).toBeTruthy()
  expect(screen.queryByText(/Choose JSON file/i)).not.toBeInTheDocument()
})

test('switches between paste and file modes', async () => {
  render(JsonToToon)

  // Initially in paste mode
  expect(screen.getByPlaceholderText('Paste your JSON here...')).toBeTruthy()

  // Switch to file mode
  const fileButton = screen.getByRole('button', { name: '📁 Upload File' })
  fireEvent.click(fileButton)

  await waitFor(() => {
    expect(screen.queryByPlaceholderText('Paste your JSON here...')).not.toBeInTheDocument()
    expect(screen.getByText(/Choose JSON file/i)).toBeTruthy()
  })

  // Switch back to paste mode
  const pasteButton = screen.getByRole('button', { name: '📝 Paste JSON' })
  fireEvent.click(pasteButton)

  await waitFor(() => {
    expect(screen.getByPlaceholderText('Paste your JSON here...')).toBeTruthy()
  })
})

test('convert button is disabled when JSON is empty', () => {
  render(JsonToToon)

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  expect(convertButton).toBeDisabled()
})

test('convert button is enabled when JSON is provided', async () => {
  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  expect(convertButton).not.toBeDisabled()
})

test('validates JSON before conversion', async () => {
  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{ invalid json }' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText(/Invalid JSON/i)).toBeTruthy()
  })

  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('successfully converts JSON to TOON', async () => {
  const mockResponse = {
    data: {
      toon: 'user:\n  active: true\n  id: 123\n  name: Ada',
      json_tokens: 0,
      toon_tokens: 0,
      token_savings: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"user": {"id": 123, "name": "Ada", "active": true}}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('TOON Output')).toBeTruthy()
  })

  const toonOutput = screen.getByText(/user:/)
  expect(toonOutput).toBeTruthy()
  expect(toonOutput.textContent).toContain('Ada')
})

test('displays token statistics when count_tokens is enabled', async () => {
  const mockResponse = {
    data: {
      toon: 'user:\n  name: Ada',
      json_tokens: 100,
      toon_tokens: 60,
      token_savings: 40.0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(JsonToToon)

  // Open advanced options
  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })
  fireEvent.click(toggleButton)

  // Enable token counting
  const countTokensCheckbox = screen.getByLabelText(
    /Count tokens \(may slow down conversion for large documents\)/i
  )
  fireEvent.click(countTokensCheckbox)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"user": {"name": "Ada"}}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('JSON Tokens:')).toBeTruthy()
    expect(screen.getByText('100')).toBeTruthy()
    expect(screen.getByText('TOON Tokens:')).toBeTruthy()
    expect(screen.getByText('60')).toBeTruthy()
    expect(screen.getByText('Savings:')).toBeTruthy()
    expect(screen.getByText('40.0%')).toBeTruthy()
  })
})

test('does not display token statistics when count_tokens is disabled', async () => {
  const mockResponse = {
    data: {
      toon: 'user:\n  name: Ada',
      json_tokens: 0,
      toon_tokens: 0,
      token_savings: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"user": {"name": "Ada"}}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('TOON Output')).toBeTruthy()
  })

  expect(screen.queryByText(/JSON Tokens:/i)).not.toBeInTheDocument()
  expect(screen.queryByText(/TOON Tokens:/i)).not.toBeInTheDocument()
})

test('displays error message on API failure', async () => {
  const mockError = {
    response: {
      data: {
        error: 'Invalid JSON provided'
      }
    }
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Invalid JSON provided')).toBeTruthy()
  })
})

test('displays generic error message when error format is unexpected', async () => {
  const mockError = {
    message: 'Network error'
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Network error')).toBeTruthy()
  })
})

test('toggles advanced options', async () => {
  render(JsonToToon)

  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })

  // Initially hidden
  expect(
    screen.queryByText(
      /Count tokens \(may slow down conversion for large documents\)/i
    )
  ).not.toBeInTheDocument()

  // Click to show
  fireEvent.click(toggleButton)
  expect(
    screen.getByText(
      /Count tokens \(may slow down conversion for large documents\)/i
    )
  ).toBeTruthy()

  // Click to hide
  fireEvent.click(toggleButton)
  await waitFor(() => {
    expect(
      screen.queryByText(
        /Count tokens \(may slow down conversion for large documents\)/i
      )
    ).not.toBeInTheDocument()
  })
})

test('sends correct request data with JSON body (paste mode)', async () => {
  const mockResponse = {
    data: {
      toon: 'test: value',
      json_tokens: 0,
      toon_tokens: 0,
      token_savings: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })

  const callArgs = mockedAxios.post.mock.calls[0]
  expect(callArgs[0]).toBe('json-to-toon')
  expect(callArgs[1]).toEqual({
    json: '{"test": "value"}',
    count_tokens: false
  })
})

test('sends correct request data with FormData (file mode)', async () => {
  const mockResponse = {
    data: {
      toon: 'test: value',
      json_tokens: 0,
      toon_tokens: 0,
      token_savings: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  // Mock FileReader
  class MockFileReader {
    result = ''
    onload: ((e: any) => void) | null = null
    onerror: (() => void) | null = null

    readAsText(file: File) {
      // Use setImmediate or Promise.resolve to ensure async behavior
      Promise.resolve().then(() => {
        if (this.onload) {
          this.onload({ target: { result: '{"test": "value"}' } })
        }
      })
    }
  }

  global.FileReader = MockFileReader as any

  render(JsonToToon)

  // Switch to file mode
  const fileButton = screen.getByRole('button', { name: '📁 Upload File' })
  fireEvent.click(fileButton)

  await waitFor(() => {
    expect(screen.getByText(/Choose JSON file/i)).toBeTruthy()
  })

  // Mock file selection
  const fileInput = screen.getByLabelText(/Choose JSON file/i) as HTMLInputElement
  const file = new File(['{"test": "value"}'], 'test.json', {
    type: 'application/json'
  })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  // Wait for file to be read and validated
  await waitFor(
    () => {
      expect(screen.getByText(/test\.json/i)).toBeTruthy()
    },
    { timeout: 2000 }
  )

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })

  const callArgs = mockedAxios.post.mock.calls[0]
  expect(callArgs[0]).toBe('json-to-toon')
  expect(callArgs[1]).toBeInstanceOf(FormData)
  expect(callArgs[2]?.headers?.['Content-Type']).toBe('multipart/form-data')
})

test('formats JSON when format button is clicked', async () => {
  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test":"value","nested":{"key":"val"}}' }
  })

  const formatButton = screen.getByRole('button', { name: 'Format JSON' })
  fireEvent.click(formatButton)

  await waitFor(() => {
    const formatted = jsonInput.value
    expect(formatted).toContain('"test"')
    expect(formatted).toContain('"value"')
    // Should be formatted with indentation
    expect(formatted).toMatch(/\s{2}/) // Contains 2 spaces (indentation)
  })
})

test('format button only appears in paste mode', async () => {
  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  // Format button should appear in paste mode
  expect(screen.getByRole('button', { name: 'Format JSON' })).toBeTruthy()

  // Switch to file mode
  const fileButton = screen.getByRole('button', { name: '📁 Upload File' })
  fireEvent.click(fileButton)

  await waitFor(() => {
    expect(screen.queryByRole('button', { name: 'Format JSON' })).not.toBeInTheDocument()
  })
})

test('clears all data when clear button is clicked', async () => {
  const mockResponse = {
    data: {
      toon: 'test: value',
      json_tokens: 0,
      toon_tokens: 0,
      token_savings: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('TOON Output')).toBeTruthy()
  })

  const clearButton = screen.getByRole('button', { name: 'Clear' })
  fireEvent.click(clearButton)

  await waitFor(() => {
    expect(jsonInput.value).toBe('')
  })

  // TOON Output section should show placeholder after clear
  await waitFor(() => {
    expect(screen.getByText('Converted TOON will appear here')).toBeTruthy()
  })
})

test('downloads TOON file when download button is clicked', async () => {
  const mockResponse = {
    data: {
      toon: 'user:\n  name: Ada',
      json_tokens: 0,
      toon_tokens: 0,
      token_savings: 0
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

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"user": {"name": "Ada"}}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('TOON Output')).toBeTruthy()
  })

  const downloadButton = screen.getByRole('button', {
    name: 'Download TOON file'
  })
  fireEvent.click(downloadButton)

  // Verify download was triggered
  expect(global.URL.createObjectURL).toHaveBeenCalled()
  expect(mockClick).toHaveBeenCalled()

  // Restore original
  document.createElement = originalCreateElement
})

test('convert button shows loading state', async () => {
  mockedAxios.post.mockImplementation(() => new Promise(() => {})) // Never resolves

  render(JsonToToon)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Converting...' })).toBeTruthy()
  })
  expect(jsonInput).toBeDisabled()
})

test('handles file upload with invalid JSON', async () => {
  // Mock FileReader
  class MockFileReader {
    result = ''
    onload: ((e: any) => void) | null = null
    onerror: (() => void) | null = null

    readAsText(file: File) {
      setTimeout(() => {
        if (this.onload) {
          this.onload({ target: { result: 'invalid json' } })
        }
      }, 0)
    }
  }

  global.FileReader = MockFileReader as any

  render(JsonToToon)

  // Switch to file mode
  const fileButton = screen.getByRole('button', { name: '📁 Upload File' })
  fireEvent.click(fileButton)

  await waitFor(() => {
    expect(screen.getByText(/Choose JSON file/i)).toBeTruthy()
  })

  const fileInput = screen.getByLabelText(/Choose JSON file/i) as HTMLInputElement
  const file = new File(['invalid json'], 'test.json', {
    type: 'application/json'
  })

  await fireEvent.change(fileInput, { target: { files: [file] } })

  await waitFor(() => {
    expect(screen.getByText(/Invalid JSON/i)).toBeTruthy()
  })
})

test('sends count_tokens in request when enabled', async () => {
  const mockResponse = {
    data: {
      toon: 'test: value',
      json_tokens: 10,
      toon_tokens: 6,
      token_savings: 40.0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(JsonToToon)

  // Open advanced options
  const toggleButton = screen.getByRole('button', { name: /Advanced Options/i })
  fireEvent.click(toggleButton)

  // Enable token counting
  const countTokensCheckbox = screen.getByLabelText(
    /Count tokens \(may slow down conversion for large documents\)/i
  )
  fireEvent.click(countTokensCheckbox)

  const jsonInput = screen.getByPlaceholderText('Paste your JSON here...')
  await fireEvent.input(jsonInput, {
    target: { value: '{"test": "value"}' }
  })

  const convertButton = screen.getByRole('button', { name: 'Convert' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'json-to-toon',
      expect.objectContaining({
        json: '{"test": "value"}',
        count_tokens: true
      })
    )
  })
})

