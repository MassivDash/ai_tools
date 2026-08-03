import { expect, test, vi, afterEach } from 'vitest'
import {
  formatToolName,
  generateMessageId,
  cleanTextForSpeech
} from './formatting'

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

test('formatToolName title-cases each underscore-separated segment', () => {
  expect(formatToolName('read_file')).toBe('Read File')
  expect(formatToolName('list_directory_contents')).toBe(
    'List Directory Contents'
  )
})

test('formatToolName leaves a single lowercase word capitalised only at the front', () => {
  expect(formatToolName('calculator')).toBe('Calculator')
  expect(formatToolName('httpRequest')).toBe('HttpRequest')
})

test('formatToolName handles empty string and leading/trailing underscores', () => {
  expect(formatToolName('')).toBe('')
  expect(formatToolName('_leading')).toBe(' Leading')
  expect(formatToolName('trailing_')).toBe('Trailing ')
})

test('generateMessageId combines the current timestamp with a random suffix', () => {
  vi.useFakeTimers()
  vi.setSystemTime(new Date('2024-01-01T00:00:00.000Z'))
  const expectedNow = new Date('2024-01-01T00:00:00.000Z').getTime()

  const id = generateMessageId()

  expect(id.startsWith(`${expectedNow}-`)).toBe(true)
  const suffix = id.slice(String(expectedNow).length + 1)
  expect(suffix.length).toBeGreaterThan(0)
  expect(suffix).toMatch(/^[0-9a-z]+$/)
})

test('generateMessageId produces distinct ids within the same millisecond', () => {
  vi.useFakeTimers()
  vi.setSystemTime(new Date('2024-01-01T00:00:00.000Z'))

  const ids = new Set(Array.from({ length: 50 }, () => generateMessageId()))

  // Timestamp is frozen, so uniqueness must come from the random suffix.
  expect(ids.size).toBe(50)
})

test('cleanTextForSpeech returns an empty string for falsy input', () => {
  expect(cleanTextForSpeech('')).toBe('')
  expect(cleanTextForSpeech(undefined as unknown as string)).toBe('')
  expect(cleanTextForSpeech(null as unknown as string)).toBe('')
})

test('cleanTextForSpeech strips markdown headers', () => {
  expect(cleanTextForSpeech('# Title\n## Subtitle')).toBe('Title Subtitle')
})

test('cleanTextForSpeech strips bold and italic markers but keeps the words', () => {
  expect(cleanTextForSpeech('**bold** and *italic*')).toBe('bold and italic')
  expect(cleanTextForSpeech('__bold__ and _italic_')).toBe('bold and italic')
})

test('cleanTextForSpeech replaces fenced code blocks with the phrase "Code block"', () => {
  expect(cleanTextForSpeech('before\n```js\nconst a = 1\n```\nafter')).toBe(
    'before Code block after'
  )
})

test('cleanTextForSpeech unwraps inline code', () => {
  expect(cleanTextForSpeech('run `npm test` now')).toBe('run npm test now')
})

test('cleanTextForSpeech keeps link text and drops the url', () => {
  expect(
    cleanTextForSpeech('see [the docs](https://example.com/x) please')
  ).toBe('see the docs please')
})

test('cleanTextForSpeech reduces images to their alt text', () => {
  // Known quirk: the link rule ([text](url) -> text) requires a non-empty label
  // and runs before the image rule, so for an image *with* alt text it wins and
  // leaves the leading "!" behind.
  expect(cleanTextForSpeech('![a cat](cat.png)')).toBe('!a cat')
  // With an empty alt the link rule cannot match, so the image rule applies and
  // removes the whole construct.
  expect(cleanTextForSpeech('![](cat.png)')).toBe('')
})

test('cleanTextForSpeech removes html tags', () => {
  expect(cleanTextForSpeech('<div class="x">hello</div>')).toBe('hello')
})

test('cleanTextForSpeech removes emoji', () => {
  expect(cleanTextForSpeech('done 🎉 shipped 🚀')).toBe('done shipped')
})

test('cleanTextForSpeech collapses whitespace and trims', () => {
  expect(cleanTextForSpeech('  a \n\n  b\t\tc  ')).toBe('a b c')
})
