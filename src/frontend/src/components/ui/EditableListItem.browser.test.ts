/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import EditableListItem from './EditableListItem.svelte'

const enterEditMode = async () => {
  await fireEvent.click(screen.getByTitle('Rename'))
  return (await waitFor(() => screen.getByRole('textbox'))) as HTMLInputElement
}

test('renders the title without a model badge when no model is given', () => {
  const { container } = render(EditableListItem, {
    props: { title: 'My conversation' }
  })

  expect(screen.getByText('My conversation')).toBeTruthy()
  expect(container.querySelector('.model-badge')).not.toBeInTheDocument()
  expect(container.querySelector('.content')).toHaveAttribute(
    'title',
    'My conversation'
  )
})

test('renders the model badge and a composite tooltip when model is given', () => {
  const { container } = render(EditableListItem, {
    props: { title: 'My conversation', model: 'llama-3-8b' }
  })

  expect(container.querySelector('.model-badge')).toHaveTextContent(
    'llama-3-8b'
  )
  expect(container.querySelector('.content')).toHaveAttribute(
    'title',
    'My conversation (llama-3-8b)'
  )
})

test('applies the active class only when active is true', () => {
  const { container, unmount } = render(EditableListItem, {
    props: { title: 'Item' }
  })
  expect(container.querySelector('.item')).not.toHaveClass('active')
  unmount()

  const second = render(EditableListItem, {
    props: { title: 'Item', active: true }
  })
  expect(second.container.querySelector('.item')).toHaveClass('active')
})

test('dispatches click when the row is clicked', async () => {
  const onClick = vi.fn()
  const { container } = render(EditableListItem, {
    props: { title: 'Item' },
    events: { click: onClick }
  })

  await fireEvent.click(container.querySelector('.item') as HTMLElement)

  expect(onClick).toHaveBeenCalledTimes(1)
})

test('dispatches click on Enter keypress but ignores other keys', async () => {
  const onClick = vi.fn()
  const { container } = render(EditableListItem, {
    props: { title: 'Item' },
    events: { click: onClick }
  })
  const row = container.querySelector('.item') as HTMLElement

  await fireEvent.keyPress(row, { key: 'a', code: 'KeyA', charCode: 97 })
  expect(onClick).not.toHaveBeenCalled()

  await fireEvent.keyPress(row, { key: 'Enter', code: 'Enter', charCode: 13 })
  expect(onClick).toHaveBeenCalledTimes(1)
})

test('hides the rename button when allowEdit is false', () => {
  render(EditableListItem, {
    props: { title: 'Item', allowEdit: false }
  })

  expect(screen.queryByTitle('Rename')).not.toBeInTheDocument()
  expect(screen.getByTitle('Delete')).toBeTruthy()
})

test('hides the delete button when allowDelete is false', () => {
  render(EditableListItem, {
    props: { title: 'Item', allowDelete: false }
  })

  expect(screen.queryByTitle('Delete')).not.toBeInTheDocument()
  expect(screen.getByTitle('Rename')).toBeTruthy()
})

test('rename switches to edit mode prefilled with the title and swallows the row click', async () => {
  const onClick = vi.fn()
  const { container } = render(EditableListItem, {
    props: { title: 'My conversation' },
    events: { click: onClick }
  })

  const input = await enterEditMode()

  expect(input.value).toBe('My conversation')
  expect(screen.queryByTitle('Rename')).not.toBeInTheDocument()
  // startEdit stops propagation so the row's click handler must not have run
  expect(onClick).not.toHaveBeenCalled()

  // while editing, clicking the row is ignored as well
  await fireEvent.click(container.querySelector('.item') as HTMLElement)
  expect(onClick).not.toHaveBeenCalled()
})

test('Enter in the edit input dispatches save with the new value and leaves edit mode', async () => {
  const onSave = vi.fn()
  render(EditableListItem, {
    props: { title: 'Old name' },
    events: { save: onSave }
  })

  const input = await enterEditMode()
  await fireEvent.input(input, { target: { value: 'New name' } })
  await fireEvent.keyPress(input, {
    key: 'Enter',
    code: 'Enter',
    charCode: 13
  })

  expect(onSave).toHaveBeenCalledTimes(1)
  expect(onSave.mock.calls[0][0].detail).toBe('New name')

  await waitFor(() => {
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })
  // the component does not rename itself -- the parent owns `title`
  expect(screen.getByText('Old name')).toBeTruthy()
})

test('non-Enter keys in the edit input neither save nor close the editor', async () => {
  const onSave = vi.fn()
  render(EditableListItem, {
    props: { title: 'Old name' },
    events: { save: onSave }
  })

  const input = await enterEditMode()
  await fireEvent.input(input, { target: { value: 'Half typed' } })
  await fireEvent.keyPress(input, { key: 'x', code: 'KeyX', charCode: 120 })

  expect(onSave).not.toHaveBeenCalled()
  expect(screen.getByRole('textbox')).toBeTruthy()
})

test('saving an unchanged value closes the editor without dispatching save', async () => {
  const onSave = vi.fn()
  render(EditableListItem, {
    props: { title: 'Same name' },
    events: { save: onSave }
  })

  const input = await enterEditMode()
  // only surrounding whitespace added -> trimmed value equals the title
  await fireEvent.input(input, { target: { value: '  Same name  ' } })
  await fireEvent.keyPress(input, {
    key: 'Enter',
    code: 'Enter',
    charCode: 13
  })

  expect(onSave).not.toHaveBeenCalled()
  await waitFor(() => {
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })
  expect(screen.getByText('Same name')).toBeTruthy()
})

test('blurring the edit input saves the pending value', async () => {
  const onSave = vi.fn()
  render(EditableListItem, {
    props: { title: 'Old name' },
    events: { save: onSave }
  })

  const input = await enterEditMode()
  await fireEvent.input(input, { target: { value: 'Blurred name' } })
  await fireEvent.blur(input)

  expect(onSave).toHaveBeenCalledTimes(1)
  expect(onSave.mock.calls[0][0].detail).toBe('Blurred name')
  await waitFor(() => {
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })
})

test('delete asks for confirmation and dispatches delete on Yes', async () => {
  const onDelete = vi.fn()
  const onClick = vi.fn()
  const { container } = render(EditableListItem, {
    props: { title: 'Item' },
    events: { delete: onDelete, click: onClick }
  })

  await fireEvent.click(screen.getByTitle('Delete'))

  await waitFor(() => {
    expect(screen.getByText('Delete?')).toBeTruthy()
  })
  expect(screen.queryByTitle('Rename')).not.toBeInTheDocument()
  expect(onDelete).not.toHaveBeenCalled()

  // clicking the row while confirming must not select the item
  await fireEvent.click(container.querySelector('.item') as HTMLElement)
  expect(onClick).not.toHaveBeenCalled()

  await fireEvent.click(screen.getByText('Yes'))

  expect(onDelete).toHaveBeenCalledTimes(1)
  expect(onClick).not.toHaveBeenCalled()
  await waitFor(() => {
    expect(screen.queryByText('Delete?')).not.toBeInTheDocument()
    expect(screen.getByTitle('Rename')).toBeTruthy()
  })
})

test('No cancels the delete confirmation without dispatching delete', async () => {
  const onDelete = vi.fn()
  render(EditableListItem, {
    props: { title: 'Item' },
    events: { delete: onDelete }
  })

  await fireEvent.click(screen.getByTitle('Delete'))
  await waitFor(() => {
    expect(screen.getByText('Delete?')).toBeTruthy()
  })

  await fireEvent.click(screen.getByText('No'))

  expect(onDelete).not.toHaveBeenCalled()
  await waitFor(() => {
    expect(screen.queryByText('Delete?')).not.toBeInTheDocument()
    expect(screen.getByText('Item')).toBeTruthy()
  })
})
