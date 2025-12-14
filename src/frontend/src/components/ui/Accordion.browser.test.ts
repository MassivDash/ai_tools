/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test } from 'vitest'
import Accordion from './Accordion.svelte'

test('renders accordion with title', () => {
  render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  expect(screen.getByText('Test Accordion')).toBeTruthy()
  expect(screen.getByRole('button')).toBeTruthy()
})

test('accordion is closed by default', () => {
  render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  const button = screen.getByRole('button')
  expect(button).toHaveAttribute('aria-expanded', 'false')
  expect(screen.queryByText(/accordion-content/i)).not.toBeInTheDocument()
})

test('accordion opens when clicked', async () => {
  render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  const button = screen.getByRole('button')
  expect(button).toHaveAttribute('aria-expanded', 'false')

  fireEvent.click(button)

  await waitFor(() => {
    expect(button).toHaveAttribute('aria-expanded', 'true')
  })
})

test('accordion closes when clicked again', async () => {
  render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  const button = screen.getByRole('button')
  
  // Open it
  fireEvent.click(button)
  await waitFor(() => {
    expect(button).toHaveAttribute('aria-expanded', 'true')
  })

  // Close it
  fireEvent.click(button)
  await waitFor(() => {
    expect(button).toHaveAttribute('aria-expanded', 'false')
  })
})

test('accordion starts open when open prop is true', () => {
  render(Accordion, {
    props: { title: 'Test Accordion', open: true }
  })

  const button = screen.getByRole('button')
  expect(button).toHaveAttribute('aria-expanded', 'true')
})

test('accordion syncs with external open prop changes', async () => {
  const { rerender } = render(Accordion, {
    props: { title: 'Test Accordion', open: false }
  })

  const button = screen.getByRole('button')
  expect(button).toHaveAttribute('aria-expanded', 'false')

  // Change open prop to true
  rerender({ title: 'Test Accordion', open: true })

  await waitFor(() => {
    expect(button).toHaveAttribute('aria-expanded', 'true')
  })

  // Change open prop back to false
  rerender({ title: 'Test Accordion', open: false })

  await waitFor(() => {
    expect(button).toHaveAttribute('aria-expanded', 'false')
  })
})

test('icon has open class when accordion is open', async () => {
  render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  const button = screen.getByRole('button')
  const icon = button.querySelector('.accordion-icon')
  
  expect(icon).not.toHaveClass('open')

  fireEvent.click(button)

  await waitFor(() => {
    expect(icon).toHaveClass('open')
  })
})

test('icon does not have open class when accordion is closed', () => {
  render(Accordion, {
    props: { title: 'Test Accordion', open: false }
  })

  const button = screen.getByRole('button')
  const icon = button.querySelector('.accordion-icon')
  
  expect(icon).not.toHaveClass('open')
})

test('renders content area when open', async () => {
  render(Accordion, {
    props: { title: 'Test Accordion', open: true }
  })

  const button = screen.getByRole('button')
  expect(button).toHaveAttribute('aria-expanded', 'true')
  
  // Check that content div exists
  const content = document.querySelector('.accordion-content')
  expect(content).toBeTruthy()
})

test('does not render content when closed', () => {
  render(Accordion, {
    props: { title: 'Test Accordion', open: false }
  })

  const content = document.querySelector('.accordion-content')
  expect(content).not.toBeInTheDocument()
})

test('button has correct classes', () => {
  render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  const button = screen.getByRole('button')
  expect(button).toHaveClass('accordion-header')
  
  const title = button.querySelector('.accordion-title')
  expect(title).toBeTruthy()
  expect(title).toHaveTextContent('Test Accordion')
  
  const icon = button.querySelector('.accordion-icon')
  expect(icon).toBeTruthy()
  expect(icon).toHaveTextContent('▼')
})

test('accordion container has correct class', () => {
  const { container } = render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  const accordion = container.querySelector('.accordion')
  expect(accordion).toBeTruthy()
})

test('can toggle multiple times', async () => {
  render(Accordion, {
    props: { title: 'Test Accordion' }
  })

  const button = screen.getByRole('button')
  
  // Toggle multiple times
  for (let i = 0; i < 3; i++) {
    fireEvent.click(button)
    await waitFor(() => {
      const expectedState = i % 2 === 0 ? 'true' : 'false'
      expect(button).toHaveAttribute('aria-expanded', expectedState)
    })
  }
})

test('external open prop changes override user interaction', async () => {
  const { rerender } = render(Accordion, {
    props: { title: 'Test Accordion', open: false }
  })

  const button = screen.getByRole('button')
  expect(button).toHaveAttribute('aria-expanded', 'false')

  // User clicks to open
  fireEvent.click(button)
  await waitFor(() => {
    expect(button).toHaveAttribute('aria-expanded', 'true')
  })

  // External prop changes to false - $effect syncs this to isOpen
  rerender({ title: 'Test Accordion', open: false })
  await waitFor(() => {
    expect(button).toHaveAttribute('aria-expanded', 'false')
  })
})

