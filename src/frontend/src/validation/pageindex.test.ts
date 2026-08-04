/**
 * @vitest-environment jsdom
 */

import { expect, test } from 'vitest'
import { validatePdfFileType, PdfFileValidationSchema } from './pageindex'

const file = (name: string, type: string) => new File(['data'], name, { type })

test('accepts a file whose MIME type is application/pdf', () => {
  expect(validatePdfFileType(file('book.pdf', 'application/pdf'))).toBe(true)
})

test('accepts a .pdf extension even when the MIME type is missing or wrong', () => {
  // Browsers sometimes report an empty or generic type; the extension is the fallback.
  expect(validatePdfFileType(file('book.pdf', ''))).toBe(true)
  expect(
    validatePdfFileType(file('book.pdf', 'application/octet-stream'))
  ).toBe(true)
})

test('extension check is case-insensitive', () => {
  expect(validatePdfFileType(file('BOOK.PDF', ''))).toBe(true)
  expect(validatePdfFileType(file('Book.Pdf', ''))).toBe(true)
})

test('rejects a non-pdf file', () => {
  expect(validatePdfFileType(file('notes.txt', 'text/plain'))).toBe(false)
  expect(
    validatePdfFileType(file('sheet.xlsx', 'application/vnd.ms-excel'))
  ).toBe(false)
})

test('rejects a name that merely contains .pdf without ending in it', () => {
  expect(validatePdfFileType(file('book.pdf.exe', ''))).toBe(false)
  expect(validatePdfFileType(file('pdf', ''))).toBe(false)
})

test('schema parses a valid pdf File', () => {
  const f = file('book.pdf', 'application/pdf')
  const result = PdfFileValidationSchema.safeParse(f)

  expect(result.success).toBe(true)
  expect(result.data).toBe(f)
})

test('schema rejects a non-pdf File with the user-facing message', () => {
  const result = PdfFileValidationSchema.safeParse(
    file('notes.txt', 'text/plain')
  )

  expect(result.success).toBe(false)
  expect(result.error?.issues[0].message).toBe(
    'File type not supported. Only PDF files are allowed.'
  )
})

test('schema rejects values that are not File instances', () => {
  expect(PdfFileValidationSchema.safeParse('book.pdf').success).toBe(false)
  expect(PdfFileValidationSchema.safeParse(null).success).toBe(false)
  expect(
    PdfFileValidationSchema.safeParse({
      name: 'book.pdf',
      type: 'application/pdf'
    }).success
  ).toBe(false)
})
