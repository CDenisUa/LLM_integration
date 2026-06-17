// Core
import { describe, expect, it } from 'vitest'
// Utils
import { bookStatusLabel, formatDuration } from './format'

describe('formatDuration', () => {
  it('formats minutes and seconds', () => {
    expect(formatDuration(0)).toBe('0:00')
    expect(formatDuration(5)).toBe('0:05')
    expect(formatDuration(65)).toBe('1:05')
    expect(formatDuration(600)).toBe('10:00')
  })

  it('formats hours', () => {
    expect(formatDuration(3661)).toBe('1:01:01')
  })

  it('guards invalid input', () => {
    expect(formatDuration(null)).toBe('0:00')
    expect(formatDuration(-5)).toBe('0:00')
    expect(formatDuration(Number.NaN)).toBe('0:00')
  })
})

describe('bookStatusLabel', () => {
  it('maps known statuses', () => {
    expect(bookStatusLabel('ready_for_generation')).toBe('Ready to generate')
    expect(bookStatusLabel('generated')).toBe('Generated')
  })

  it('falls back to raw value', () => {
    expect(bookStatusLabel('weird_state')).toBe('weird_state')
  })
})
