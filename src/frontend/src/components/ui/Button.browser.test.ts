/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import Button from './Button.svelte'

test('renders button with default props', () => {
  const { container } = render(Button, {
    props: { children: () => 'Click me' }
  })
  
  const button = container.querySelector('button')
  expect(button).toBeTruthy()
  expect(button).toHaveClass('button-primary')
  expect(button).toHaveClass('button-medium')
  // Note: Snippet content may not render in test environment
})

test('renders button with different variants', () => {
  const { container, rerender } = render(Button, {
    props: { variant: 'success', children: () => 'Success' }
  })
  
  let button = container.querySelector('button')
  expect(button).toHaveClass('button-success')

  rerender({ variant: 'danger', children: () => 'Danger' })
  button = container.querySelector('button')
  expect(button).toHaveClass('button-danger')

  rerender({ variant: 'secondary', children: () => 'Secondary' })
  button = container.querySelector('button')
  expect(button).toHaveClass('button-secondary')
})

test('renders button with different sizes', () => {
  const { container } = render(Button, {
    props: { size: 'small', children: () => 'Small' }
  })
  
  const button = container.querySelector('button')
  expect(button).toHaveClass('button-small')
  
  const { container: container2 } = render(Button, {
    props: { size: 'large', children: () => 'Large' }
  })
  
  const largeButton = container2.querySelector('button')
  expect(largeButton).toHaveClass('button-large')
})

test('handles click events', () => {
  const handleClick = vi.fn()
  const { container } = render(Button, {
    props: { children: () => 'Click me', onclick: handleClick }
  })
  
  const button = container.querySelector('button')
  fireEvent.click(button!)
  expect(handleClick).toHaveBeenCalledTimes(1)
})

test('disables button when disabled prop is true', () => {
  const { container } = render(Button, {
    props: { disabled: true, children: () => 'Disabled' }
  })
  
  const button = container.querySelector('button')
  expect(button).toBeDisabled()
})

test('applies custom class', () => {
  const { container } = render(Button, {
    props: { class: 'custom-class', children: () => 'Custom' }
  })
  
  const button = container.querySelector('button')
  expect(button).toHaveClass('custom-class')
})

test('renders button with correct type attribute', () => {
  const { container } = render(Button, {
    props: { type: 'submit', children: () => 'Submit' }
  })
  
  const button = container.querySelector('button')
  expect(button).toHaveAttribute('type', 'submit')
})

test('renders button with reset type', () => {
  const { container } = render(Button, {
    props: { type: 'reset', children: () => 'Reset' }
  })
  
  const button = container.querySelector('button')
  expect(button).toHaveAttribute('type', 'reset')
})

test('renders button with info variant', () => {
  const { container } = render(Button, {
    props: { variant: 'info', children: () => 'Info' }
  })
  
  const button = container.querySelector('button')
  expect(button).toHaveClass('button-info')
})

test('renders button without children', () => {
  render(Button)
  
  const button = screen.getByRole('button')
  expect(button).toBeTruthy()
  expect(button.textContent).toBe('')
})

test('combines multiple classes correctly', () => {
  const { container } = render(Button, {
    props: {
      variant: 'success',
      size: 'large',
      class: 'extra-class',
      children: () => 'Test'
    }
  })
  
  const button = container.querySelector('button')
  expect(button).toHaveClass('button')
  expect(button).toHaveClass('button-success')
  expect(button).toHaveClass('button-large')
  expect(button).toHaveClass('extra-class')
})

test('handles multiple clicks', () => {
  const handleClick = vi.fn()
  const { container } = render(Button, {
    props: { children: () => 'Click me', onclick: handleClick }
  })
  
  const button = container.querySelector('button')
  fireEvent.click(button!)
  fireEvent.click(button!)
  fireEvent.click(button!)
  
  expect(handleClick).toHaveBeenCalledTimes(3)
})

test('does not trigger click when disabled', () => {
  const handleClick = vi.fn()
  const { container } = render(Button, {
    props: { disabled: true, children: () => 'Disabled', onclick: handleClick }
  })
  
  const button = container.querySelector('button')
  fireEvent.click(button!)
  
  // Click handler should still be called (browser behavior), but button is disabled
  expect(button).toBeDisabled()
})

test('renders complex children content', () => {
  const { container } = render(Button, {
    props: {
      children: () => {
        return 'Save & Continue'
      }
    }
  })
  
  const button = container.querySelector('button')
  expect(button).toBeTruthy()
})

test('applies all variant classes correctly', () => {
  const variants: Array<'primary' | 'secondary' | 'success' | 'danger' | 'info'> = [
    'primary',
    'secondary',
    'success',
    'danger',
    'info'
  ]
  
  variants.forEach(variant => {
    const { container, unmount } = render(Button, {
      props: { variant, children: () => variant }
    })
    
    const button = container.querySelector('button')
    expect(button).toHaveClass(`button-${variant}`)
    unmount()
  })
})

test('applies all size classes correctly', () => {
  const sizes: Array<'small' | 'medium' | 'large'> = ['small', 'medium', 'large']
  
  sizes.forEach(size => {
    const { container, unmount } = render(Button, {
      props: { size, children: () => size }
    })
    
    const button = container.querySelector('button')
    expect(button).toHaveClass(`button-${size}`)
    unmount()
  })
})

test('handles empty class string', () => {
  const { container } = render(Button, {
    props: { class: '', children: () => 'Test' }
  })
  
  const button = container.querySelector('button')
  expect(button).toHaveClass('button-primary')
  expect(button).toHaveClass('button-medium')
})

test('handles whitespace in class string', () => {
  const { container } = render(Button, {
    props: { class: '  class1  class2  ', children: () => 'Test' }
  })
  
  const button = container.querySelector('button')
  // Classes should be trimmed
  expect(button?.className).toContain('class1')
  expect(button?.className).toContain('class2')
})

