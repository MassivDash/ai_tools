import { expect, test } from 'vitest'
import { ContestantJoinSchema } from './oneOfTen'

test('accepts a valid contestant and trims the name', () => {
  const result = ContestantJoinSchema.safeParse({ name: '  Ada  ', age: 30 })

  expect(result.success).toBe(true)
  expect(result.data).toEqual({ name: 'Ada', age: 30 })
})

test('coerces a numeric string age to a number', () => {
  const result = ContestantJoinSchema.safeParse({ name: 'Ada', age: '30' })

  expect(result.success).toBe(true)
  expect(result.data?.age).toBe(30)
  expect(typeof result.data?.age).toBe('number')
})

test('rejects an empty name with the configured message', () => {
  const result = ContestantJoinSchema.safeParse({ name: '', age: 30 })

  expect(result.success).toBe(false)
  expect(result.error?.issues[0].message).toBe('Name is required')
  expect(result.error?.issues[0].path).toEqual(['name'])
})

test('rejects a whitespace-only name (trim happens before the min check)', () => {
  const result = ContestantJoinSchema.safeParse({ name: '   ', age: 30 })

  expect(result.success).toBe(false)
  expect(result.error?.issues[0].message).toBe('Name is required')
})

test('rejects a non-positive age', () => {
  expect(ContestantJoinSchema.safeParse({ name: 'Ada', age: 0 }).success).toBe(
    false
  )
  const negative = ContestantJoinSchema.safeParse({ name: 'Ada', age: -1 })
  expect(negative.success).toBe(false)
  expect(negative.error?.issues[0].message).toBe('Age must be valid')
})

test('accepts the age boundaries 1 and 120 but rejects 121', () => {
  expect(ContestantJoinSchema.safeParse({ name: 'Ada', age: 1 }).success).toBe(
    true
  )
  expect(
    ContestantJoinSchema.safeParse({ name: 'Ada', age: 120 }).success
  ).toBe(true)

  const tooOld = ContestantJoinSchema.safeParse({ name: 'Ada', age: 121 })
  expect(tooOld.success).toBe(false)
  expect(tooOld.error?.issues[0].message).toBe('Age must be realistic')
})

test('rejects a fractional age', () => {
  const result = ContestantJoinSchema.safeParse({ name: 'Ada', age: 30.5 })

  expect(result.success).toBe(false)
  expect(result.error?.issues[0].path).toEqual(['age'])
})

test('rejects an unparseable age', () => {
  expect(
    ContestantJoinSchema.safeParse({ name: 'Ada', age: 'thirty' }).success
  ).toBe(false)
})

test('rejects a missing name or age outright', () => {
  expect(ContestantJoinSchema.safeParse({ age: 30 }).success).toBe(false)
  expect(ContestantJoinSchema.safeParse({ name: 'Ada' }).success).toBe(false)
})
