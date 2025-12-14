/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import TextToTokens from './textToTokens.svelte'
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
})

test('renders component with initial state', () => {
  render(TextToTokens)

  expect(screen.getByText('Text to Tokens Converter')).toBeTruthy()
  expect(
    screen.getByPlaceholderText('Enter or paste text to count tokens...')
  ).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Count Tokens' })).toBeTruthy()
  expect(screen.getByText(/💡 Tip: Press Ctrl\+Enter/)).toBeTruthy()
})

test('convert button is disabled when text is empty', () => {
  render(TextToTokens)

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  expect(convertButton).toBeDisabled()
})

test('convert button is enabled when text is provided', async () => {
  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Hello world' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  expect(convertButton).not.toBeDisabled()
})

test('convert button shows loading state', async () => {
  mockedAxios.post.mockImplementation(() => new Promise(() => {})) // Never resolves

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Hello world' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Counting...' })).toBeTruthy()
  })
  expect(textInput).toBeDisabled()
})

test('successfully counts tokens', async () => {
  const mockResponse = {
    data: {
      token_count: 42,
      character_count: 150,
      word_count: 25
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, {
    target: { value: 'This is a test text for token counting.' }
  })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Token Count Results:')).toBeTruthy()
  })

  expect(screen.getByText('42')).toBeTruthy()
  expect(screen.getByText('25')).toBeTruthy()
  expect(screen.getByText('150')).toBeTruthy()
  expect(screen.getByText('GPT-2 tokenizer count')).toBeTruthy()
  expect(screen.getByText('Whitespace-separated words')).toBeTruthy()
  expect(screen.getByText('Total character count')).toBeTruthy()
})

test('displays formatted numbers with locale string', async () => {
  const mockResponse = {
    data: {
      token_count: 1234,
      character_count: 5678,
      word_count: 901
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test text' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Token Count Results:')).toBeTruthy()
  })

  // Numbers should be formatted (locale string may add commas)
  expect(screen.getByText(/1[,.]?234/)).toBeTruthy()
  expect(screen.getByText(/5[,.]?678/)).toBeTruthy()
  expect(screen.getByText(/901/)).toBeTruthy()
})

test('displays error message on API failure', async () => {
  const mockError = {
    response: {
      data: {
        error: 'Text cannot be empty'
      }
    }
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Text cannot be empty')).toBeTruthy()
  })
})

test('displays generic error message when error format is unexpected', async () => {
  const mockError = {
    message: 'Network error'
  }

  mockedAxios.post.mockRejectedValue(mockError)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Network error')).toBeTruthy()
  })
})

test('triggers conversion on Ctrl+Enter key press', async () => {
  const mockResponse = {
    data: {
      token_count: 10,
      character_count: 50,
      word_count: 8
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test text' } })

  fireEvent.keyDown(textInput, { key: 'Enter', ctrlKey: true })

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })
})

test('triggers conversion on Cmd+Enter key press (Mac)', async () => {
  const mockResponse = {
    data: {
      token_count: 10,
      character_count: 50,
      word_count: 8
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test text' } })

  fireEvent.keyDown(textInput, { key: 'Enter', metaKey: true })

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })
})

test('does not trigger conversion on Enter alone', async () => {
  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test text' } })

  fireEvent.keyDown(textInput, { key: 'Enter' })

  // Wait a bit to ensure no API call is made
  await new Promise((resolve) => setTimeout(resolve, 100))

  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('sends correct request data', async () => {
  const mockResponse = {
    data: {
      token_count: 10,
      character_count: 50,
      word_count: 8
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: '  Test text  ' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })

  expect(mockedAxios.post).toHaveBeenCalledWith('text-to-tokens', {
    text: 'Test text' // Should be trimmed
  })
})

test('shows error when trying to count empty text', async () => {
  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: '   ' } }) // Only whitespace

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  expect(convertButton).toBeDisabled()
})

test('clears previous results on new conversion', async () => {
  const mockResponse1 = {
    data: {
      token_count: 10,
      character_count: 50,
      word_count: 8
    }
  }

  const mockResponse2 = {
    data: {
      token_count: 20,
      character_count: 100,
      word_count: 15
    }
  }

  mockedAxios.post
    .mockResolvedValueOnce(mockResponse1)
    .mockResolvedValueOnce(mockResponse2)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )

  // First conversion
  await fireEvent.input(textInput, { target: { value: 'First text' } })
  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('10')).toBeTruthy()
  })

  // Second conversion
  await fireEvent.input(textInput, { target: { value: 'Second text' } })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('20')).toBeTruthy()
    expect(screen.queryByText('10')).not.toBeInTheDocument()
  })
})

test('displays all three stat cards with correct labels', async () => {
  const mockResponse = {
    data: {
      token_count: 100,
      character_count: 500,
      word_count: 75
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test text' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  await waitFor(() => {
    expect(screen.getByText('Tokens')).toBeTruthy()
    expect(screen.getByText('Words')).toBeTruthy()
    expect(screen.getByText('Characters')).toBeTruthy()
    expect(screen.getByText('GPT-2 tokenizer count')).toBeTruthy()
    expect(screen.getByText('Whitespace-separated words')).toBeTruthy()
    expect(screen.getByText('Total character count')).toBeTruthy()
  })
})

test('does not display results when token count is 0', async () => {
  const mockResponse = {
    data: {
      token_count: 0,
      character_count: 0,
      word_count: 0
    }
  }

  mockedAxios.post.mockResolvedValue(mockResponse)

  render(TextToTokens)

  const textInput = screen.getByPlaceholderText(
    'Enter or paste text to count tokens...'
  )
  await fireEvent.input(textInput, { target: { value: 'Test' } })

  const convertButton = screen.getByRole('button', { name: 'Count Tokens' })
  fireEvent.click(convertButton)

  // Wait a bit to ensure component has processed
  await new Promise((resolve) => setTimeout(resolve, 100))

  // Results should not be displayed when token_count is 0
  expect(screen.queryByText('Token Count Results:')).not.toBeInTheDocument()
})

