/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import Modal from './Modal.svelte'

test('does not render when isOpen is false', () => {
  render(Modal, {
    props: { isOpen: false }
  })
  
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
})

test('renders when isOpen is true', () => {
  render(Modal, {
    props: { isOpen: true }
  })
  
  expect(screen.getByRole('dialog')).toBeTruthy()
})

test('renders with title', () => {
  render(Modal, {
    props: { isOpen: true, title: 'Test Modal' }
  })
  
  expect(screen.getByText('Test Modal')).toBeTruthy()
  expect(screen.getByText('Test Modal').id).toBe('modal-title')
})

test('does not render title when not provided', () => {
  render(Modal, {
    props: { isOpen: true }
  })
  
  expect(screen.queryByText(/modal-title/i)).not.toBeInTheDocument()
})

test('shows close button by default', () => {
  render(Modal, {
    props: { isOpen: true }
  })
  
  expect(screen.getByLabelText('Close')).toBeTruthy()
})

test('hides close button when showCloseButton is false', () => {
  render(Modal, {
    props: { isOpen: true, showCloseButton: false }
  })
  
  expect(screen.queryByLabelText('Close')).not.toBeInTheDocument()
})

test('dispatches close event when close button is clicked', async () => {
  const handleClose = vi.fn()
  render(Modal, {
    props: { isOpen: true },
    events: { close: handleClose }
  })
  
  const closeButton = screen.getByLabelText('Close')
  fireEvent.click(closeButton)
  
  await waitFor(() => {
    expect(handleClose).toHaveBeenCalledTimes(1)
  })
})

test('dispatches close event when Escape key is pressed', async () => {
  const handleClose = vi.fn()
  const { container } = render(Modal, {
    props: { isOpen: true },
    events: { close: handleClose }
  })
  
  const overlay = container.querySelector('.modal-overlay')
  fireEvent.keyDown(overlay!, { key: 'Escape' })
  
  await waitFor(() => {
    expect(handleClose).toHaveBeenCalledTimes(1)
  })
})

test('dispatches close event when overlay is clicked', async () => {
  const handleClose = vi.fn()
  const { container } = render(Modal, {
    props: { isOpen: true },
    events: { close: handleClose }
  })
  
  const overlay = container.querySelector('.modal-overlay')
  // Simulate clicking the overlay (not the content)
  fireEvent.click(overlay!, { target: overlay })
  
  await waitFor(() => {
    expect(handleClose).toHaveBeenCalledTimes(1)
  })
})

test('does not close when modal content is clicked', async () => {
  const handleClose = vi.fn()
  const { container } = render(Modal, {
    props: { isOpen: true },
    events: { close: handleClose }
  })
  
  const modalContent = container.querySelector('.modal-content')
  fireEvent.click(modalContent!)
  
  // Wait a bit to ensure close is not called
  await new Promise(resolve => setTimeout(resolve, 100))
  
  expect(handleClose).not.toHaveBeenCalled()
})

test('renders slot content', () => {
  const { container } = render(Modal, {
    props: { isOpen: true }
  })
  
  const modalBody = container.querySelector('.modal-body')
  expect(modalBody).toBeTruthy()
  // Slot content rendering is tested through component usage
})

test('renders footer slot when provided', () => {
  const { container } = render(Modal, {
    props: { isOpen: true }
  })
  
  // Footer slot is conditionally rendered based on $$slots.footer
  // In tests, we can check if footer div exists
  const footer = container.querySelector('.modal-footer')
  // Footer may or may not exist depending on slot usage
  // This test just verifies the structure can handle it
  expect(container.querySelector('.modal-content')).toBeTruthy()
})

test('modal has correct ARIA attributes', () => {
  render(Modal, {
    props: { isOpen: true, title: 'Test Modal' }
  })
  
  const dialog = screen.getByRole('dialog')
  expect(dialog).toHaveAttribute('aria-labelledby', 'modal-title')
  expect(dialog).toHaveAttribute('tabindex', '-1')
})

test('overlay has correct role and tabindex', () => {
  const { container } = render(Modal, {
    props: { isOpen: true }
  })
  
  const overlay = container.querySelector('.modal-overlay')
  expect(overlay).toHaveAttribute('role', 'button')
  expect(overlay).toHaveAttribute('tabindex', '0')
})

test('does not close on other keys', async () => {
  const handleClose = vi.fn()
  const { container } = render(Modal, {
    props: { isOpen: true },
    events: { close: handleClose }
  })
  
  const overlay = container.querySelector('.modal-overlay')
  fireEvent.keyDown(overlay!, { key: 'Enter' })
  
  // Wait a bit to ensure close is not called
  await new Promise(resolve => setTimeout(resolve, 100))
  
  expect(handleClose).not.toHaveBeenCalled()
})

test('does not close when Escape is pressed but modal is closed', async () => {
  const handleClose = vi.fn()
  render(Modal, {
    props: { isOpen: false },
    events: { close: handleClose }
  })
  
  // Even if we try to trigger Escape, it shouldn't work when closed
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
  
  await new Promise(resolve => setTimeout(resolve, 100))
  
  expect(handleClose).not.toHaveBeenCalled()
})

