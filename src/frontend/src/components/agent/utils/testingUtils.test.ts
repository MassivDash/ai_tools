/**
 * @vitest-environment jsdom
 */

import { afterEach, describe, expect, test } from 'vitest'
import { utils, write } from 'xlsx'
import { parseQuestionsFromFile } from './testingUtils.ts'

const workbookFile = (
  sheet: ReturnType<typeof utils.aoa_to_sheet>,
  name = 'questions.xlsx'
): File => {
  const workbook = utils.book_new()
  utils.book_append_sheet(workbook, sheet, 'Sheet1')
  const buffer = write(workbook, { type: 'array', bookType: 'xlsx' })
  return new File([buffer], name)
}

const fileFromRows = (rows: Record<string, unknown>[]): File =>
  workbookFile(utils.json_to_sheet(rows))

const fileFromGrid = (grid: unknown[][]): File =>
  workbookFile(utils.aoa_to_sheet(grid))

const OriginalFileReader = globalThis.FileReader

afterEach(() => {
  globalThis.FileReader = OriginalFileReader
})

describe('parseQuestionsFromFile', () => {
  test('reads a "questions" column and trims each value', async () => {
    const file = fileFromRows([
      { questions: '  What is the revenue?  ' },
      { questions: 'Who signed the lease?' }
    ])
    await expect(parseQuestionsFromFile(file)).resolves.toEqual([
      'What is the revenue?',
      'Who signed the lease?'
    ])
  })

  test.each([['Question'], ['QUESTIONS'], ['Text'], ['content']])(
    'accepts %s as the header, case-insensitively',
    async (header) => {
      const file = fileFromGrid([[header], ['first'], ['second']])
      await expect(parseQuestionsFromFile(file)).resolves.toEqual([
        'first',
        'second'
      ])
    }
  )

  test('picks the question column out of a wider sheet', async () => {
    const file = fileFromGrid([
      ['id', 'measure', 'questions'],
      [1, 'ignored', 'the real question']
    ])
    await expect(parseQuestionsFromFile(file)).resolves.toEqual([
      'the real question'
    ])
  })

  test('stringifies numeric answers', async () => {
    const file = fileFromGrid([['questions'], [42], [3.5]])
    await expect(parseQuestionsFromFile(file)).resolves.toEqual(['42', '3.5'])
  })

  test('skips blank and whitespace-only cells', async () => {
    const file = fileFromGrid([
      ['questions'],
      ['keep me'],
      ['   '],
      ['also me']
    ])
    await expect(parseQuestionsFromFile(file)).resolves.toEqual([
      'keep me',
      'also me'
    ])
  })

  test('skips values that are neither string nor number', async () => {
    const file = fileFromRows([
      { questions: true },
      { questions: 'a real question' }
    ])
    await expect(parseQuestionsFromFile(file)).resolves.toEqual([
      'a real question'
    ])
  })

  test('skips rows that do not carry the question column at all', async () => {
    const file = fileFromGrid([
      ['questions', 'notes'],
      [undefined, 'orphan note'],
      ['kept', 'note']
    ])
    await expect(parseQuestionsFromFile(file)).resolves.toEqual(['kept'])
  })

  test('rejects when no recognised header is present', async () => {
    const file = fileFromGrid([['measure'], ['some value']])
    await expect(parseQuestionsFromFile(file)).rejects.toThrow(
      'No valid questions found. Ensure column header is "questions", "question", or "text".'
    )
  })

  test('rejects when the question column exists but holds nothing usable', async () => {
    const file = fileFromGrid([['questions'], ['   '], ['']])
    await expect(parseQuestionsFromFile(file)).rejects.toThrow(
      /No valid questions found/
    )
  })

  test('rejects with the underlying parser error when the workbook is corrupt', async () => {
    // A truncated ZIP local-file header: sniffed as xlsx, then fails to unzip
    const corrupt = new Uint8Array([0x50, 0x4b, 0x03, 0x04, 0x00, 0x01, 0x02])
    const file = new File([corrupt], 'broken.xlsx')

    await expect(parseQuestionsFromFile(file)).rejects.toSatisfy(
      (err: unknown) =>
        err instanceof Error && !/No valid questions found/.test(err.message)
    )
  })

  test('rejects when the FileReader itself errors', async () => {
    const readerError = new Error('read failed')

    class FailingFileReader {
      onload: ((_event: unknown) => void) | null = null
      onerror: ((_event: unknown) => void) | null = null
      readAsArrayBuffer() {
        this.onerror?.(readerError)
      }
    }
    globalThis.FileReader = FailingFileReader as unknown as typeof FileReader

    await expect(
      parseQuestionsFromFile(fileFromGrid([['questions'], ['a']]))
    ).rejects.toBe(readerError)
  })
})
