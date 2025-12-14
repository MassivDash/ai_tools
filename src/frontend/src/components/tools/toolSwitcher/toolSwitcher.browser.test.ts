/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test } from 'vitest'
import ToolSwitcher from './toolSwitcher.svelte'

// Note: UrlToMarkdown will be rendered but we'll test the structure

test('renders tool switcher with header', () => {
  render(ToolSwitcher)

  expect(screen.getByText('AI Tools')).toBeTruthy()
  expect(screen.getByText('Select a tool to get started')).toBeTruthy()
})

test('renders all tool cards when no tool is selected', () => {
  render(ToolSwitcher)

  expect(screen.getByText('URL to Markdown')).toBeTruthy()
  expect(screen.getByText('HTML to Markdown')).toBeTruthy()
  expect(screen.getByText('PDF to Markdown')).toBeTruthy()

  expect(screen.getByText('Convert web pages to markdown format')).toBeTruthy()
  expect(screen.getByText('Paste HTML and convert to markdown')).toBeTruthy()
  expect(
    screen.getByText('Upload PDF files and convert to markdown')
  ).toBeTruthy()
})

test('selects tool when tool card is clicked', async () => {
  render(ToolSwitcher)

  const urlToolCard = screen.getByText('URL to Markdown').closest('button')
  fireEvent.click(urlToolCard!)

  await waitFor(
    () => {
      const backButton = screen.queryByText('← Back to Tools')
      // Tool should be selected, showing back button
      expect(backButton).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows back button when tool is selected', async () => {
  render(ToolSwitcher)

  const urlToolCard = screen.getByText('URL to Markdown').closest('button')
  fireEvent.click(urlToolCard!)

  await waitFor(
    () => {
      const backButton = screen.queryByText('← Back to Tools')
      expect(backButton).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('returns to tool list when back button is clicked', async () => {
  render(ToolSwitcher)

  // Select a tool
  const urlToolCard = screen.getByText('URL to Markdown').closest('button')
  fireEvent.click(urlToolCard!)

  await waitFor(
    () => {
      expect(screen.queryByText('← Back to Tools')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Click back button
  const backButton = screen.getByText('← Back to Tools')
  fireEvent.click(backButton)

  await waitFor(
    () => {
      expect(screen.queryByText('← Back to Tools')).not.toBeInTheDocument()
      // All tool cards should be visible again
      expect(screen.getByText('URL to Markdown')).toBeTruthy()
      expect(screen.getByText('HTML to Markdown')).toBeTruthy()
      expect(screen.getByText('PDF to Markdown')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('displays URL to Markdown component when selected', async () => {
  const { container } = render(ToolSwitcher)

  const urlToolCard = screen.getByText('URL to Markdown').closest('button')
  fireEvent.click(urlToolCard!)

  await waitFor(
    () => {
      // The component should be rendered
      const toolContent = container.querySelector('.tool-content')
      expect(toolContent).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('displays placeholder for HTML to Markdown', async () => {
  render(ToolSwitcher)

  const htmlToolCard = screen.getByText('HTML to Markdown').closest('button')
  fireEvent.click(htmlToolCard!)

  await waitFor(() => {
    expect(
      screen.getByText('🚧 HTML to Markdown tool coming soon')
    ).toBeTruthy()
    expect(
      screen.getByText(/This tool will allow you to paste HTML content/)
    ).toBeTruthy()
  })
})

test('displays placeholder for PDF to Markdown', async () => {
  render(ToolSwitcher)

  const pdfToolCard = screen.getByText('PDF to Markdown').closest('button')
  fireEvent.click(pdfToolCard!)

  await waitFor(() => {
    expect(screen.getByText('🚧 PDF to Markdown tool coming soon')).toBeTruthy()
    expect(
      screen.getByText(/This tool will allow you to upload PDF files/)
    ).toBeTruthy()
  })
})

test('displays correct tool name in header when tool is selected', async () => {
  render(ToolSwitcher)

  const htmlToolCard = screen.getByText('HTML to Markdown').closest('button')
  fireEvent.click(htmlToolCard!)

  await waitFor(() => {
    // Should show the tool name in the header
    const headers = screen.getAllByText('HTML to Markdown')
    expect(headers.length).toBeGreaterThan(0)
  })
})

test('can switch between different tools', async () => {
  render(ToolSwitcher)

  // Select first tool
  const urlToolCard = screen.getByText('URL to Markdown').closest('button')
  fireEvent.click(urlToolCard!)

  await waitFor(
    () => {
      expect(screen.queryByText('← Back to Tools')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Go back
  const backButton = screen.getByText('← Back to Tools')
  fireEvent.click(backButton)

  await waitFor(
    () => {
      expect(screen.queryByText('← Back to Tools')).not.toBeInTheDocument()
    },
    { timeout: 2000 }
  )

  // Select different tool
  const pdfToolCard = screen.getByText('PDF to Markdown').closest('button')
  fireEvent.click(pdfToolCard!)

  await waitFor(
    () => {
      expect(
        screen.getByText('🚧 PDF to Markdown tool coming soon')
      ).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('renders tool icons', () => {
  render(ToolSwitcher)

  // Icons are emojis, so we check for the tool cards
  const toolCards = document.querySelectorAll('.tool-card')
  expect(toolCards.length).toBe(3)

  toolCards.forEach((card) => {
    const icon = card.querySelector('.tool-icon')
    expect(icon).toBeTruthy()
  })
})

test('tool cards have correct structure', () => {
  const { container } = render(ToolSwitcher)

  const toolCards = container.querySelectorAll('.tool-card')
  expect(toolCards.length).toBe(3)

  toolCards.forEach((card) => {
    expect(card.querySelector('.tool-icon')).toBeTruthy()
    expect(card.querySelector('.tool-name')).toBeTruthy()
    expect(card.querySelector('.tool-description')).toBeTruthy()
  })
})

test('tool container is shown when tool is selected', async () => {
  const { container } = render(ToolSwitcher)

  const urlToolCard = screen.getByText('URL to Markdown').closest('button')
  fireEvent.click(urlToolCard!)

  await waitFor(
    () => {
      const toolContainer = container.querySelector('.tool-container')
      expect(toolContainer).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('tool grid is hidden when tool is selected', async () => {
  const { container } = render(ToolSwitcher)

  // Initially visible
  const toolGrid = container.querySelector('.tools-grid')
  expect(toolGrid).toBeTruthy()

  // Select a tool
  const urlToolCard = screen.getByText('URL to Markdown').closest('button')
  fireEvent.click(urlToolCard!)

  await waitFor(
    () => {
      // Grid should be hidden
      const grid = container.querySelector('.tools-grid')
      expect(grid).not.toBeInTheDocument()
    },
    { timeout: 2000 }
  )
})
